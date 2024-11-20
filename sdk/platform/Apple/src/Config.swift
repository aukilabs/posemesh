extension Config {
    public var serveAsBootstrap: Bool {
        get {
            return __serveAsBootstrap()
        }
        set {
            __setServeAsBootstrap(newValue)
        }
    }

    public var serveAsRelay: Bool {
        get {
            return __serveAsRelay()
        }
        set {
            __setServeAsRelay(newValue)
        }
    }

    public var bootstraps: [String] {
        get {
            return __bootstraps()
        }
    }

    public var relays: [String] {
        get {
            return __relays()
        }
    }

    public static var `default`: Config {
        get {
            return __default()
        }
    }
}
