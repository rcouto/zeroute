//! Single-owner native engine loop. Android routing/TUN is deliberately absent
//! from this bring-up stage, so root/configuration observations never imply VPN readiness.

use super::{STATE, State};
use crate::network::Configuration;
use crate::packet::Bridge;
use crate::readiness::{Attempt, Observation};
use crate::settings;
use std::ffi::c_void;
use std::ffi::{CStr, c_char};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6, UdpSocket};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

#[repr(C)]
struct Callbacks {
    put: extern "C" fn(*mut c_void, i32, u64, u64, *const u8, i32),
    get: extern "C" fn(*mut c_void, i32, u64, u64, *mut u8, u32) -> i32,
    send: extern "C" fn(*mut c_void, i32, *const u8, u32, u32, *const u8, u32) -> i32,
    event: extern "C" fn(*mut c_void, i32),
    configuration: extern "C" fn(*mut c_void, u64, i32, u64, u32, *const c_char),
    frame: extern "C" fn(*mut c_void, u64, u64, u64, u32, *const u8, u32),
}

unsafe extern "C" {
    fn zr_core_version() -> u64;
    fn zr_node_new(callbacks: *const Callbacks, user: *mut c_void, now: i64) -> *mut c_void;
    fn zr_node_delete(node: *mut c_void);
    fn zr_node_address(node: *mut c_void) -> u64;
    fn zr_node_join(node: *mut c_void, network: u64) -> i32;
    fn zr_node_receive(
        node: *mut c_void,
        now: i64,
        family: i32,
        address: *const u8,
        port: u32,
        scope: u32,
        data: *const u8,
        length: u32,
        deadline: *mut i64,
    ) -> i32;
    fn zr_node_frame(
        node: *mut c_void,
        now: i64,
        network: u64,
        source: u64,
        destination: u64,
        kind: u32,
        data: *const u8,
        length: u32,
        deadline: *mut i64,
    ) -> i32;
    fn zr_node_subscribe(node: *mut c_void, network: u64, mac: u64, adi: u32, enable: i32) -> i32;
    fn zr_node_background(node: *mut c_void, now: i64, deadline: *mut i64) -> i32;
}

enum Command {
    Attach { fd: OwnedFd, revision: u64 },
}
#[derive(Default)]
struct Plane {
    reads: u64,
    writes: u64,
    bridge: Option<Bridge>,
    tun: Option<OwnedFd>,
    revision: u64,
}

struct Running {
    commands: SyncSender<Command>,
    stop: Arc<AtomicBool>,
    wake: Arc<OwnedFd>,
    thread: JoinHandle<()>,
}
static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

struct Context {
    root: PathBuf,
    network: u64,
    attempt: Attempt,
    ipv4: UdpSocket,
    ipv6: Option<UdpSocket>,
    stop: Arc<AtomicBool>,
    failed: AtomicBool,
    storage: Mutex<()>,
    plane: Mutex<Plane>,
    urgent: AtomicBool,
}

struct Node(NonNull<c_void>);
impl Drop for Node {
    fn drop(&mut self) {
        // SAFETY: this node is confined to its owner thread. Context and callbacks
        // remain alive until deletion returns, including final state callbacks.
        unsafe { zr_node_delete(self.0.as_ptr()) };
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub(super) fn version() -> String {
    // SAFETY: version lookup is stateless and has no pointer arguments.
    let value = unsafe { zr_core_version() };
    format!(
        "{}.{}.{}",
        value >> 32,
        (value >> 16) & 0xffff,
        value & 0xffff
    )
}

fn update(context: &Context, action: impl FnOnce(&mut State)) {
    if let Ok(mut state) = STATE.lock() {
        // observe() performs the actual attempt check; state metadata is only
        // written while this attempt is accepted by the connection model.
        if state.connection.accepts(context.attempt) {
            action(&mut state);
        }
    }
}

fn fail(context: &Context, message: &'static str) {
    context.failed.store(true, Ordering::Release);
    context.stop.store(true, Ordering::Release);
    update(context, |state| state.error = Some(message));
}

pub(super) fn start(root: &Path, protect: &mut dyn FnMut(i32) -> bool) -> Result<(), &'static str> {
    let network = settings::load(root)
        .map_err(|_| "Saved network settings are invalid.")?
        .ok_or("Save a network ID first.")?
        .value();
    let mut running = RUNNING.lock().map_err(|_| "Engine state unavailable.")?;
    if let Some(previous) = running.as_ref() {
        if !previous.thread.is_finished() {
            return Err("The engine is already running.");
        }
    }
    if let Some(previous) = running.take() {
        let _ = previous.thread.join();
    }
    let ipv4 = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .map_err(|_| "Could not open the IPv4 transport.")?;
    ipv4.set_nonblocking(true)
        .map_err(|_| "Could not configure the IPv4 transport.")?;
    let ipv6 = UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0)).ok();
    if let Some(socket) = &ipv6 {
        socket
            .set_nonblocking(true)
            .map_err(|_| "Could not configure the IPv6 transport.")?;
    }
    if !protect(ipv4.as_raw_fd())
        || ipv6
            .as_ref()
            .is_some_and(|socket| !protect(socket.as_raw_fd()))
    {
        return Err("Android could not protect the VPN transport sockets.");
    }
    // SAFETY: eventfd returns a new owned descriptor or -1. It is only adopted on success.
    let wake_fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
    if wake_fd < 0 {
        return Err("Could not create the engine wakeup signal.");
    }
    // SAFETY: this is the unique owner of the successfully created descriptor.
    let wake = Arc::new(unsafe { OwnedFd::from_raw_fd(wake_fd) });
    let stop = Arc::new(AtomicBool::new(false));
    let attempt = {
        let mut state = STATE.lock().map_err(|_| "Connection state unavailable.")?;
        state.address = 0;
        state.diagnostics.clear();
        state.configuration = None;
        state.error = None;
        state.connection.start()
    };
    let context = Box::new(Context {
        root: root.join("engine"),
        network,
        attempt,
        ipv4,
        ipv6,
        stop: stop.clone(),
        failed: AtomicBool::new(false),
        storage: Mutex::new(()),
        plane: Mutex::new(Plane::default()),
        urgent: AtomicBool::new(false),
    });
    let (commands, receiver) = mpsc::sync_channel(8);
    let worker_wake = wake.clone();
    let thread = thread::Builder::new()
        .name("zeroute-core".into())
        .spawn(move || {
            if catch_unwind(AssertUnwindSafe(|| run(&context, &worker_wake, &receiver))).is_err() {
                fail(&context, "The engine stopped unexpectedly.");
            }
            update(&context, |state| {
                state.connection.stop();
                state.configuration = None;
            });
        })
        .map_err(|_| {
            if let Ok(mut state) = STATE.lock() {
                state.connection.stop();
            }
            "Could not start the engine worker."
        })?;
    *running = Some(Running {
        stop,
        wake,
        thread,
        commands,
    });
    Ok(())
}

pub(super) fn stop() -> Result<(), &'static str> {
    let mut owner = RUNNING.lock().map_err(|_| "Engine state unavailable.")?;
    let running = owner.take();
    if let Ok(mut state) = STATE.lock() {
        state.connection.stop();
        state.configuration = None;
        state.error = None;
    }
    if let Some(running) = running {
        running.stop.store(true, Ordering::Release);
        let signal = 1_u64;
        // SAFETY: the descriptor is owned through Arc, and signal points to eight initialized bytes.
        unsafe {
            libc::write(running.wake.as_raw_fd(), (&signal as *const u64).cast(), 8);
        }
        running
            .thread
            .join()
            .map_err(|_| "The engine worker stopped unexpectedly.")?;
    }
    drop(owner);
    Ok(())
}

fn run(context: &Context, wake: &OwnedFd, commands: &Receiver<Command>) {
    let callbacks = Callbacks {
        put: state_put,
        get: state_get,
        send: wire_send,
        event,
        configuration,
        frame: virtual_frame,
    };
    // SAFETY: context is boxed and outlives this node; callbacks are kept on this
    // stack until node deletion. All callbacks contain panics at the ABI boundary.
    let pointer = unsafe {
        zr_node_new(
            &callbacks,
            (context as *const Context).cast_mut().cast(),
            now(),
        )
    };
    let Some(pointer) = NonNull::new(pointer) else {
        fail(context, "ZeroTier core initialization failed.");
        return;
    };
    let node = Node(pointer);
    if context.failed.load(Ordering::Acquire) {
        return;
    }
    // SAFETY: this owner thread exclusively holds the live node.
    let address = unsafe { zr_node_address(node.0.as_ptr()) };
    update(context, |state| state.address = address);
    // SAFETY: the saved network ID is validated and the node remains live.
    if unsafe { zr_node_join(node.0.as_ptr(), context.network) } != 0 {
        fail(context, "The network join request failed.");
        return;
    }
    let mut deadline = 0_i64;
    let mut buffer = [0_u8; 65536];
    let mut subscriptions = Vec::new();
    while !context.stop.load(Ordering::Acquire) {
        if now() >= deadline {
            // SAFETY: node and the output deadline are valid for this synchronous call.
            if unsafe { zr_node_background(node.0.as_ptr(), now(), &mut deadline) } != 0 {
                fail(context, "ZeroTier background processing failed.");
                break;
            }
        }
        drive(context, &node, commands, &mut subscriptions, &mut deadline);
        if context.stop.load(Ordering::Acquire) {
            break;
        }
        let (tun_fd, tun_events) = context
            .plane
            .lock()
            .map(|plane| {
                (
                    plane.tun.as_ref().map_or(-1, AsRawFd::as_raw_fd),
                    libc::POLLIN
                        | if plane
                            .bridge
                            .as_ref()
                            .is_some_and(|b| !b.incoming.is_empty())
                        {
                            libc::POLLOUT
                        } else {
                            0
                        },
                )
            })
            .unwrap_or((-1, 0));
        let mut descriptors = [
            libc::pollfd {
                fd: context.ipv4.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: context.ipv6.as_ref().map_or(-1, AsRawFd::as_raw_fd),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: wake.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: tun_fd,
                events: tun_events,
                revents: 0,
            },
        ];
        let timeout = if context.urgent.swap(false, Ordering::AcqRel) {
            0
        } else {
            (deadline - now()).clamp(1, 1000) as i32
        };
        // SAFETY: descriptors is writable for its stated length; all nonnegative
        // descriptors are owned by context/wake throughout this poll call.
        let result = unsafe {
            libc::poll(
                descriptors.as_mut_ptr(),
                descriptors.len() as libc::nfds_t,
                timeout,
            )
        };
        if result < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            fail(context, "Transport polling failed.");
            break;
        }
        if descriptors[2].revents & libc::POLLIN != 0 {
            let mut signal = 0u64;
            // SAFETY: wake is live and signal is writable for eight bytes.
            unsafe {
                libc::read(wake.as_raw_fd(), (&mut signal as *mut u64).cast(), 8);
            }
        }
        if descriptors[3].revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            fail(context, "The Android VPN interface became unavailable.");
            break;
        }
        if descriptors[3].revents & libc::POLLIN != 0 {
            read_tunnel(context, tun_fd, &mut buffer);
        }
        for (index, socket) in [Some(&context.ipv4), context.ipv6.as_ref()]
            .into_iter()
            .enumerate()
        {
            let Some(socket) = socket else {
                continue;
            };
            if descriptors[index].revents & (libc::POLLHUP | libc::POLLNVAL) != 0 {
                fail(context, "The underlying network became unavailable.");
                break;
            }
            if descriptors[index].revents & libc::POLLERR != 0 {
                // Clear asynchronous UDP errors so they cannot create a busy loop.
                let _ = socket.take_error();
            }
            if descriptors[index].revents & libc::POLLIN == 0 {
                continue;
            }
            for _ in 0..128 {
                if context.stop.load(Ordering::Acquire) {
                    break;
                }
                match socket.recv_from(&mut buffer) {
                    Ok((length, source)) => {
                        let (family, address, port, scope) = endpoint(source);
                        // SAFETY: pointers reference initialized buffers for the
                        // specified lengths, and the live node is thread-confined.
                        if unsafe {
                            zr_node_receive(
                                node.0.as_ptr(),
                                now(),
                                family,
                                address.as_ptr(),
                                port,
                                scope,
                                buffer.as_ptr(),
                                length as u32,
                                &mut deadline,
                            )
                        } != 0
                        {
                            fail(context, "ZeroTier packet processing failed.");
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        fail(context, "The transport socket failed.");
                        break;
                    }
                }
            }
        }
    }
}

fn endpoint(source: SocketAddr) -> (i32, [u8; 16], u32, u32) {
    match source {
        SocketAddr::V4(address) => {
            let mut bytes = [0_u8; 16];
            bytes[..4].copy_from_slice(&address.ip().octets());
            (4, bytes, address.port().into(), 0)
        }
        SocketAddr::V6(address) => (
            6,
            address.ip().octets(),
            address.port().into(),
            address.scope_id(),
        ),
    }
}

fn callback<T>(user: *mut c_void, fallback: T, action: impl FnOnce(&Context) -> T) -> T {
    if user.is_null() {
        return fallback;
    }
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: only our C adapter calls these functions, with the boxed
        // context supplied to zr_node_new. The context outlives node deletion.
        action(unsafe { &*user.cast::<Context>() })
    }))
    .unwrap_or_else(|_| {
        // SAFETY: the same callback lifetime contract applies after unwinding.
        fail(
            unsafe { &*user.cast::<Context>() },
            "A native callback failed.",
        );
        fallback
    })
}

fn state_path(context: &Context, kind: i32, first: u64, second: u64) -> Option<PathBuf> {
    // Identity lookups initially use ID zero; using that ID as a filename would
    // regenerate the identity every startup. Keep canonical identity names.
    Some(match kind {
        1 => context.root.join("identity.public"),
        2 => context.root.join("identity.secret"),
        3 => context.root.join("planet"),
        4..=6 => context
            .root
            .join(format!("state-{kind}-{first:016x}-{second:016x}")),
        _ => return None,
    })
}

extern "C" fn state_put(
    user: *mut c_void,
    kind: i32,
    first: u64,
    second: u64,
    data: *const u8,
    length: i32,
) {
    callback(user, (), |context| {
        if context.failed.load(Ordering::Acquire) {
            return;
        }
        let Some(path) = state_path(context, kind, first, second) else {
            return;
        };
        let Ok(_guard) = context.storage.lock() else {
            fail(context, "Engine storage unavailable.");
            return;
        };
        let result = (|| -> std::io::Result<()> {
            fs::create_dir_all(&context.root)?;
            if length < 0 {
                match fs::remove_file(&path) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                    result => return result,
                }
            }
            if data.is_null() || length > 16 * 1024 * 1024 {
                return Err(std::io::Error::other("Invalid core state buffer"));
            }
            // SAFETY: the core owns this immutable callback buffer for length bytes.
            let bytes = unsafe { std::slice::from_raw_parts(data, length as usize) };
            let pending = PathBuf::from(format!("{}.pending", path.display()));
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&pending)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            fs::rename(pending, path)?;
            File::open(&context.root)?.sync_all()
        })();
        if result.is_err() && kind != 5 {
            fail(context, "Engine state could not be saved safely.");
        }
    });
}

extern "C" fn state_get(
    user: *mut c_void,
    kind: i32,
    first: u64,
    second: u64,
    data: *mut u8,
    capacity: u32,
) -> i32 {
    callback(user, -1, |context| {
        if data.is_null() || capacity > 16 * 1024 * 1024 {
            return -1;
        }
        let Some(path) = state_path(context, kind, first, second) else {
            return -1;
        };
        let mut bytes = Vec::new();
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return -1,
            Err(_) => {
                if kind == 1 || kind == 2 {
                    fail(context, "Stored identity could not be read safely.");
                }
                return -1;
            }
        };
        if file
            .take(u64::from(capacity) + 1)
            .read_to_end(&mut bytes)
            .is_err()
            || bytes.len() > capacity as usize
        {
            if kind == 1 || kind == 2 {
                fail(context, "Stored identity could not be read safely.");
            }
            return -1;
        }
        // SAFETY: data is writable for capacity bytes; bytes is a separate owned allocation.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
        }
        bytes.len() as i32
    })
}

extern "C" fn wire_send(
    user: *mut c_void,
    family: i32,
    address: *const u8,
    port: u32,
    scope: u32,
    data: *const u8,
    length: u32,
) -> i32 {
    callback(user, -1, |context| {
        if address.is_null() || data.is_null() || length > 65535 || port > 65535 {
            return -1;
        }
        if context.failed.load(Ordering::Acquire) {
            return -1;
        }
        // SAFETY: our adapter always supplies 16 address bytes and length payload bytes.
        let address = unsafe { *address.cast::<[u8; 16]>() };
        // SAFETY: this callback payload remains valid until the adapter returns.
        let bytes = unsafe { std::slice::from_raw_parts(data, length as usize) };
        let result = match family {
            4 => context.ipv4.send_to(
                bytes,
                (
                    Ipv4Addr::new(address[0], address[1], address[2], address[3]),
                    port as u16,
                ),
            ),
            6 => match &context.ipv6 {
                Some(socket) => socket.send_to(
                    bytes,
                    SocketAddrV6::new(Ipv6Addr::from(address), port as u16, 0, scope),
                ),
                None => return -1,
            },
            _ => return -1,
        };
        if result.is_ok_and(|written| written == bytes.len()) {
            0
        } else {
            -1
        }
    })
}

extern "C" fn event(user: *mut c_void, value: i32) {
    callback(user, (), |context| match value {
        0 => update(context, |state| {
            state
                .connection
                .observe(context.attempt, Observation::EngineRunning(true));
        }),
        1 | 2 => update(context, |state| {
            state
                .connection
                .observe(context.attempt, Observation::ControlPlaneOnline(value == 2));
        }),
        3 => update(context, |state| {
            state
                .connection
                .observe(context.attempt, Observation::EngineRunning(false));
        }),
        4 => fail(
            context,
            "Node identity collision detected; the engine was stopped.",
        ),
        _ => (),
    });
}

extern "C" fn configuration(
    user: *mut c_void,
    network: u64,
    status: i32,
    mac: u64,
    mtu: u32,
    text: *const c_char,
) {
    callback(user, (), |context| {
        if network != context.network || context.stop.load(Ordering::Acquire) {
            return;
        }
        let parsed = if status == 1 && !text.is_null() {
            // SAFETY: our adapter provides a terminated string valid for this callback.
            unsafe { CStr::from_ptr(text) }
                .to_str()
                .map_err(|_| "Invalid controller configuration.")
                .and_then(|text| Configuration::parse(mac, mtu, text))
        } else {
            Err(match status {
                2 => "Authorize this node in the network controller.",
                3 => "The network was not found.",
                _ => "Waiting for network configuration.",
            })
        };
        let mut replacement = None;
        update(context, |state| match parsed {
            Ok(config) => {
                if state.configuration.as_ref().map(|(_, c)| c) != Some(&config) {
                    state.revision = state
                        .revision
                        .checked_add(1)
                        .expect("configuration revision exhausted");
                    state
                        .connection
                        .observe(context.attempt, Observation::NetworkConfigured(false));
                    state.configuration = Some((state.revision, config.clone()));
                    replacement = Some((state.revision, Some(config)));
                }
                state
                    .connection
                    .observe(context.attempt, Observation::NetworkConfigured(true));
                state.error = None;
            }
            Err(error) => {
                state.configuration = None;
                state
                    .connection
                    .observe(context.attempt, Observation::NetworkConfigured(false));
                state.error = if status == 0 { None } else { Some(error) };
                replacement = Some((0, None));
            }
        });
        if let Some((revision, config)) = replacement {
            if let Ok(mut plane) = context.plane.lock() {
                plane.tun = None;
                plane.revision = revision;
                let mut seed = [0u8; 2];
                // /dev/urandom also works on API 26, before bionic exported getrandom.
                if File::open("/dev/urandom")
                    .and_then(|mut file| file.read_exact(&mut seed))
                    .is_err()
                {
                    fail(context, "Could not initialize the DNS probe.");
                    return;
                }
                plane.bridge = config.map(|config| Bridge::new(config, u16::from_ne_bytes(seed)));
            }
            context.urgent.store(true, Ordering::Release);
        }
    });
}

extern "C" fn virtual_frame(
    user: *mut c_void,
    network: u64,
    source: u64,
    destination: u64,
    kind: u32,
    data: *const u8,
    length: u32,
) {
    callback(user, (), |context| {
        if network != context.network
            || data.is_null()
            || length > 65535
            || context.stop.load(Ordering::Acquire)
        {
            return;
        }
        // SAFETY: the core provides length immutable payload bytes for this callback.
        let packet = unsafe { std::slice::from_raw_parts(data, length as usize) };
        if let Ok(mut plane) = context.plane.lock() {
            if let Some(bridge) = &mut plane.bridge {
                bridge.receive(source, destination, kind, packet, now() as u64);
            }
        }
        context.urgent.store(true, Ordering::Release);
    });
}

pub(super) fn attach_tunnel(fd: i32, revision: u64) -> Result<(), &'static str> {
    if fd < 0 {
        return Err("Invalid Android tunnel descriptor.");
    }
    // SAFETY: fcntl duplicates a borrowed Android descriptor; only its new fd is adopted.
    let copy = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
    if copy < 0 {
        return Err("Could not duplicate the Android tunnel.");
    }
    // SAFETY: copy is a fresh owned descriptor returned by fcntl.
    let owned = unsafe { OwnedFd::from_raw_fd(copy) };
    let running = RUNNING.lock().map_err(|_| "Engine state unavailable.")?;
    let running = running.as_ref().ok_or("The engine is not running.")?;
    running
        .commands
        .try_send(Command::Attach {
            fd: owned,
            revision,
        })
        .map_err(|_| "The engine cannot accept a tunnel update.")?;
    let signal = 1u64;
    // SAFETY: wake is live and signal points to eight initialized bytes.
    unsafe {
        libc::write(running.wake.as_raw_fd(), (&signal as *const u64).cast(), 8);
    }
    Ok(())
}

fn read_tunnel(context: &Context, fd: i32, buffer: &mut [u8]) {
    for _ in 0..128 {
        // SAFETY: fd is owned by this worker's plane and buffer is writable.
        let count = unsafe { libc::read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        if count < 0 {
            if matches!(
                std::io::Error::last_os_error().kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
            ) {
                break;
            }
            fail(context, "Reading the Android VPN interface failed.");
            break;
        }
        if count == 0 {
            fail(context, "The Android VPN interface closed.");
            break;
        }
        if let Ok(mut plane) = context.plane.lock() {
            plane.reads += 1;
            if let Some(bridge) = &mut plane.bridge {
                bridge.outgoing(&buffer[..count as usize], now() as u64);
            }
        }
    }
}

fn drive(
    context: &Context,
    node: &Node,
    commands: &Receiver<Command>,
    subscriptions: &mut Vec<(u64, u32)>,
    deadline: &mut i64,
) {
    let Ok(mut plane) = context.plane.lock() else {
        fail(context, "VPN state unavailable.");
        return;
    };
    while let Ok(Command::Attach { fd, revision }) = commands.try_recv() {
        if revision == plane.revision && plane.bridge.is_some() {
            plane.tun = Some(fd);
            update(context, |state| {
                state
                    .connection
                    .observe(context.attempt, Observation::TunnelEstablished(true));
            });
        }
    }
    let fd = plane.tun.as_ref().map(AsRawFd::as_raw_fd);
    let reads = plane.reads;
    let mut writes = plane.writes;
    let Some(bridge) = &mut plane.bridge else {
        return;
    };
    if fd.is_some() {
        bridge.tick(now() as u64);
    }
    if let Some(fd) = fd {
        while let Some(packet) = bridge.incoming.front() {
            // SAFETY: fd is a live worker-owned TUN and packet has its stated length.
            let count = unsafe { libc::write(fd, packet.as_ptr().cast(), packet.len()) };
            if count < 0
                && matches!(
                    std::io::Error::last_os_error().kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                )
            {
                break;
            }
            if count != packet.len() as isize {
                fail(context, "Writing the Android VPN interface failed.");
                return;
            }
            bridge.incoming.pop_front();
            writes += 1;
        }
        update(context, |state| {
            state
                .connection
                .observe(context.attempt, Observation::DnsReady(bridge.dns_ready));
        });
    }
    let diagnostic = format!(
        "Android packets read {reads} / written {writes}. {}",
        bridge.diagnostics()
    );
    update(context, |state| state.diagnostics = diagnostic);
    let desired = bridge.config.subscriptions();
    let mac = bridge.config.mac;
    let frames: Vec<_> = bridge.frames.drain(..).collect();
    plane.writes = writes;
    drop(plane); // Core calls may invoke callbacks: never retain the plane lock.
    for entry in subscriptions.iter().filter(|s| !desired.contains(s)) {
        // SAFETY: the core is confined to this thread and all values are copied.
        unsafe {
            zr_node_subscribe(node.0.as_ptr(), context.network, entry.0, entry.1, 0);
        }
    }
    for entry in desired.iter().filter(|s| !subscriptions.contains(s)) {
        // SAFETY: the core is confined to this thread and all values are copied.
        if unsafe { zr_node_subscribe(node.0.as_ptr(), context.network, entry.0, entry.1, 1) } != 0
        {
            fail(context, "Multicast subscription failed.");
            return;
        }
    }
    *subscriptions = desired;
    for frame in frames {
        // SAFETY: this live core and packet buffers are confined to the owner thread.
        if unsafe {
            zr_node_frame(
                node.0.as_ptr(),
                now(),
                context.network,
                mac,
                frame.destination,
                frame.kind,
                frame.bytes.as_ptr(),
                frame.bytes.len() as u32,
                deadline,
            )
        } != 0
        {
            fail(context, "Sending a VPN frame failed.");
            break;
        }
    }
}
