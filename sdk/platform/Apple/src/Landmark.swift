extension Landmark {
    public var type: String {
        get {
            return __type()
        }
        set {
            __setType(newValue)
        }
    }

    public var id: String {
        get {
            return __id()
        }
        set {
            __setId(newValue)
        }
    }

    public var position: Vector3 {
        get {
            return __position()
        }
        set {
            __setPosition(newValue)
        }
    }
}
