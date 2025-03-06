extension Config {
    public var bootstraps: [String] {
        get {
            return __bootstraps()
        }
        set {
            setBootstraps(newValue)
        }
    }

    public var relays: [String] {
        get {
            return __relays()
        }
        set {
            setRelays(newValue)
        }
    }

    public var privateKey: Data {
        get {
            return __privateKey()
        }
        set {
            __setPrivateKey(newValue)
        }
    }

    public var privateKeyPath: String {
        get {
            return __privateKeyPath()
        }
        set {
            __setPrivateKeyPath(newValue)
        }
    }

    public var enableMDNS: Bool {
        get {
            return __enableMDNS()
        }
        set {
            __setEnableMDNS(newValue)
        }
    }

    public var name: String {
        get {
            return __name()
        }
        set {
            __setName(newValue)
        }
    }

    public static var `default`: Config {
        get {
            return __default()
        }
    }
}
