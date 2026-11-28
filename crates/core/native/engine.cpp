// Application-owned ABI adapter; the upstream core is linked without source patches.
#include "engine.h"
#include "ZeroTierOne.h"
#include <cstring>
#include <new>
#include <string>
#include <arpa/inet.h>

struct Engine {
    ZT_Node *node{};
    zr_callbacks callbacks{};
    ZT_Node_Callbacks native_callbacks{};
    void *user{};
};

static void state_put(ZT_Node *, void *user, void *, ZT_StateObjectType kind,
                      const uint64_t id[2], const void *data, int length) {
    auto *engine = static_cast<Engine *>(user);
    engine->callbacks.put(engine->user, kind, id ? id[0] : 0, id ? id[1] : 0,
                          static_cast<const uint8_t *>(data), length);
}

static int state_get(ZT_Node *, void *user, void *, ZT_StateObjectType kind,
                     const uint64_t id[2], void *data, unsigned int length) {
    auto *engine = static_cast<Engine *>(user);
    return engine->callbacks.get(engine->user, kind, id ? id[0] : 0, id ? id[1] : 0,
                                 static_cast<uint8_t *>(data), length);
}

static int wire_send(ZT_Node *, void *user, void *, int64_t,
                     const sockaddr_storage *destination, const void *data,
                     unsigned int length, unsigned int) {
    auto *engine = static_cast<Engine *>(user);
    uint8_t address[16]{};
    unsigned int port = 0, scope = 0;
    int family = 0;
    if (destination->ss_family == AF_INET) {
        const auto *ipv4 = reinterpret_cast<const sockaddr_in *>(destination);
        std::memcpy(address, &ipv4->sin_addr, 4);
        port = ntohs(ipv4->sin_port);
        family = 4;
    } else if (destination->ss_family == AF_INET6) {
        const auto *ipv6 = reinterpret_cast<const sockaddr_in6 *>(destination);
        std::memcpy(address, &ipv6->sin6_addr, 16);
        port = ntohs(ipv6->sin6_port);
        scope = ipv6->sin6_scope_id;
        family = 6;
    } else {
        return -1;
    }
    return engine->callbacks.send(engine->user, family, address, port, scope,
                                  static_cast<const uint8_t *>(data), length);
}

static int lookup_path(ZT_Node *, void *user, void *, uint64_t peer,
                       int requested_family, sockaddr_storage *result) {
    auto *engine = static_cast<Engine *>(user);
    int family;
    switch (requested_family) {
        case -1: family = -1; break;
        case AF_INET: family = 4; break;
        case AF_INET6: family = 6; break;
        default: return 0;
    }
    uint8_t address[16]{};
    unsigned int port = 0;
    const int found = engine->callbacks.lookup_path(engine->user, peer, family, address, &port);
    *result = {};
    if (found == 4) {
        sockaddr_in endpoint{};
        endpoint.sin_family = AF_INET;
        endpoint.sin_port = htons(static_cast<uint16_t>(port));
        std::memcpy(&endpoint.sin_addr, address, 4);
        std::memcpy(result, &endpoint, sizeof(endpoint));
    } else if (found == 6) {
        sockaddr_in6 endpoint{};
        endpoint.sin6_family = AF_INET6;
        endpoint.sin6_port = htons(static_cast<uint16_t>(port));
        std::memcpy(&endpoint.sin6_addr, address, 16);
        std::memcpy(result, &endpoint, sizeof(endpoint));
    } else {
        return 0;
    }
    return 1;
}

static void virtual_frame(ZT_Node *, void *user, void *, uint64_t network, void **,
                          uint64_t source, uint64_t destination, unsigned int type, unsigned int vlan,
                          const void *data, unsigned int length) {
    auto *engine = static_cast<Engine *>(user);
    if (!vlan) engine->callbacks.frame(engine->user, network, source, destination, type,
                                       static_cast<const uint8_t *>(data), length);
}

static std::string ip_text(const sockaddr_storage &value, bool prefix) {
    char text[INET6_ADDRSTRLEN]{};
    unsigned int bits = 0;
    if (value.ss_family == AF_INET) {
        const auto &v = reinterpret_cast<const sockaddr_in &>(value);
        if (!inet_ntop(AF_INET, &v.sin_addr, text, sizeof(text))) return {};
        bits = ntohs(v.sin_port);
    } else if (value.ss_family == AF_INET6) {
        const auto &v = reinterpret_cast<const sockaddr_in6 &>(value);
        if (!inet_ntop(AF_INET6, &v.sin6_addr, text, sizeof(text))) return {};
        bits = ntohs(v.sin6_port);
    } else return {};
    return std::string(text) + (prefix ? "/" + std::to_string(bits) : "");
}

static int network_configuration(ZT_Node *, void *user, void *, uint64_t network,
                                 void **, ZT_VirtualNetworkConfigOperation operation,
                                 const ZT_VirtualNetworkConfig *config) {
    auto *engine = static_cast<Engine *>(user);
    int status = config ? static_cast<int>(config->status) : 0;
    if (operation == ZT_VIRTUAL_NETWORK_CONFIG_OPERATION_DOWN ||
        operation == ZT_VIRTUAL_NETWORK_CONFIG_OPERATION_DESTROY) status = 0;
    std::string text;
    if (config && status == ZT_NETWORK_STATUS_OK) {
        for (unsigned int i = 0; i < config->assignedAddressCount && i < ZT_MAX_ZT_ASSIGNED_ADDRESSES; ++i)
            text += "a " + ip_text(config->assignedAddresses[i], true) + "\n";
        for (unsigned int i = 0; i < config->routeCount && i < ZT_MAX_NETWORK_ROUTES; ++i) {
            auto via = ip_text(config->routes[i].via, false);
            text += "r " + ip_text(config->routes[i].target, true) + " " + (via.empty() ? "-" : via) + "\n";
        }
        for (unsigned int i = 0; i < ZT_MAX_DNS_SERVERS; ++i) {
            auto server = ip_text(config->dns.server_addr[i], false);
            if (!server.empty()) text += "d " + server + "\n";
        }
        if (config->dns.domain[0]) text += "s " + std::string(config->dns.domain, strnlen(config->dns.domain, sizeof(config->dns.domain))) + "\n";
    }
    engine->callbacks.configuration(engine->user, network, status, config ? config->mac : 0,
                                     config ? config->mtu : 0, text.c_str());
    return 0;
}

static void event(ZT_Node *, void *user, void *, ZT_Event value, const void *) {
    auto *engine = static_cast<Engine *>(user);
    // Forward only event identifiers, never trace payloads or identity data.
    switch (value) {
        case ZT_EVENT_UP: engine->callbacks.event(engine->user, 0); break;
        case ZT_EVENT_OFFLINE: engine->callbacks.event(engine->user, 1); break;
        case ZT_EVENT_ONLINE: engine->callbacks.event(engine->user, 2); break;
        case ZT_EVENT_DOWN: engine->callbacks.event(engine->user, 3); break;
        case ZT_EVENT_FATAL_ERROR_IDENTITY_COLLISION: engine->callbacks.event(engine->user, 4); break;
        default: break;
    }
}

extern "C" uint64_t zr_core_version() {
    int major = 0, minor = 0, revision = 0;
    ZT_version(&major, &minor, &revision);
    return (static_cast<uint64_t>(major) << 32) |
           (static_cast<uint64_t>(minor) << 16) | static_cast<uint64_t>(revision);
}

extern "C" void *zr_node_new(const zr_callbacks *callbacks, void *user, int64_t now) {
    auto *engine = new (std::nothrow) Engine{};
    if (!engine) return nullptr;
    engine->callbacks = *callbacks;
    engine->user = user;
    auto &native = engine->native_callbacks;
    native.version = 0;
    native.statePutFunction = state_put;
    native.stateGetFunction = state_get;
    native.wirePacketSendFunction = wire_send;
    native.virtualNetworkFrameFunction = virtual_frame;
    native.virtualNetworkConfigFunction = network_configuration;
    native.eventCallback = event;
    native.pathLookupFunction = lookup_path;
    ZT_Node_Config config{};
    if (ZT_Node_new(&engine->node, &config, engine, nullptr, &native, now) != ZT_RESULT_OK) {
        delete engine;
        return nullptr;
    }
    return engine;
}

extern "C" void zr_node_delete(void *handle) {
    auto *engine = static_cast<Engine *>(handle);
    if (engine) {
        ZT_Node_delete(engine->node);
        delete engine;
    }
}

extern "C" uint64_t zr_node_address(void *handle) {
    return ZT_Node_address(static_cast<Engine *>(handle)->node);
}

extern "C" int zr_node_join(void *handle, uint64_t network) {
    return ZT_Node_join(static_cast<Engine *>(handle)->node, network, nullptr, nullptr);
}

extern "C" int zr_node_receive(void *handle, int64_t now, int family,
                               const uint8_t *address, unsigned int port,
                               unsigned int scope, const uint8_t *data,
                               unsigned int length, int64_t *deadline) {
    sockaddr_storage remote{};
    if (family == 4) {
        auto *ipv4 = reinterpret_cast<sockaddr_in *>(&remote);
        ipv4->sin_family = AF_INET;
        ipv4->sin_port = htons(static_cast<uint16_t>(port));
        std::memcpy(&ipv4->sin_addr, address, 4);
    } else if (family == 6) {
        auto *ipv6 = reinterpret_cast<sockaddr_in6 *>(&remote);
        ipv6->sin6_family = AF_INET6;
        ipv6->sin6_port = htons(static_cast<uint16_t>(port));
        ipv6->sin6_scope_id = scope;
        std::memcpy(&ipv6->sin6_addr, address, 16);
    } else return -1;
    return ZT_Node_processWirePacket(static_cast<Engine *>(handle)->node, nullptr,
                                     now, family, &remote, data, length, deadline);
}

extern "C" int zr_node_background(void *handle, int64_t now, int64_t *deadline) {
    return ZT_Node_processBackgroundTasks(static_cast<Engine *>(handle)->node,
                                         nullptr, now, deadline);
}

extern "C" int zr_node_frame(void *handle, int64_t now, uint64_t network, uint64_t source,
                              uint64_t destination, unsigned int type, const uint8_t *data,
                              unsigned int length, int64_t *deadline) {
    return ZT_Node_processVirtualNetworkFrame(static_cast<Engine *>(handle)->node, nullptr,
        now, network, source, destination, type, 0, data, length, deadline);
}
extern "C" int zr_node_subscribe(void *handle, uint64_t network, uint64_t mac, unsigned int adi, int enable) {
    auto *node = static_cast<Engine *>(handle)->node;
    return enable ? ZT_Node_multicastSubscribe(node, nullptr, network, mac, adi)
                  : ZT_Node_multicastUnsubscribe(node, network, mac, adi);
}
