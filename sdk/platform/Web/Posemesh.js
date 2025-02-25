posemeshModule.Posemesh = {};

__internalPosemeshAPI.builderFunctions.push(function(context) {
    __internalPosemesh.Posemesh.prototype.sendMessage = function(message, peerId, protocol) {
        return __internalPosemeshBase.posemeshNetworkingContextSendMessage(this.__context, message, peerId, protocol, 0);
    };

    __internalPosemesh.Posemesh.prototype.sendString = function(string, appendTerminatingNullCharacter, peerId, protocol) {
        let message = new TextEncoder("utf-8").encode(string);
        if (appendTerminatingNullCharacter) {
            let newMessage = new Uint8Array(message.length + 1);
            newMessage.set(message, 0);
            newMessage.set(0, message.length);
            message = newMessage;
        }
        return __internalPosemeshBase.posemeshNetworkingContextSendMessage(this.__context, message, peerId, protocol, 0);
    };

    Object.assign(posemeshModule.Posemesh, __internalPosemesh.Posemesh);
});
