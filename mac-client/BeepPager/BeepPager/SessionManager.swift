//
//  Created by Cyandev on 2024/9/29.
//  Copyright (c) 2024 Cyandev. All rights reserved.
//

import Foundation
import Combine
import os

@MainActor
final class SessionManager {
    
    private struct AccountInfo {
        var userToken: String
        var secretKey: String
    }
    
    private enum ConnectionEvent: StateMachineEvent {
        case disconnected
        case connecting
        case connected
        case dataReceived(Data)
        
        enum _Type {
            case disconnected
            case connecting
            case connected
            case dataReceived
        }
        
        var eventType: _Type {
            switch self {
            case .disconnected:
                return .disconnected
            case .connecting:
                return .connecting
            case .connected:
                return .connected
            case .dataReceived(_):
                return .dataReceived
            }
        }
    }
    
    static let `default` = SessionManager()
    
    private var syncManager: SyncManager!
    
    private let connection: Connection = .init()
    private var isConnected: Bool = false
    private var connectedFuture: UnsafeFutureSubject<(), any Error>?
    
    private var accountInfo: AccountInfo?
    
    private typealias ConnectionStateMachine = StateMachine<ConnectionEvent>
    private let stateMachine: ConnectionStateMachine
    private let idleState: ConnectionStateMachine.State
    private let connectingState: ConnectionStateMachine.State
    private let signingInState: ConnectionStateMachine.State
    private let signedInState: ConnectionStateMachine.State
    private let closingState: ConnectionStateMachine.State
    
    private var cancellables: Set<AnyCancellable> = .init()
    
    private let logger: Logger = .init(subsystem: "me.cyandev.BeepPager.SessionManager", category: "general")
    
    init() {
        self.stateMachine = .init()
        self.idleState = stateMachine.addState(with: "Idle")
        self.connectingState = stateMachine.addState(with: "Connecting")
        self.signingInState = stateMachine.addState(with: "Signing In")
        self.signedInState = stateMachine.addState(with: "Signed In")
        self.closingState = stateMachine.addState(with: "Closing")
        
        self.syncManager = .init(syncCoordinator: self)
        
        prepareAndStartStateMachine()
        observeConnectionState()
    }
    
    func signIn(withUserToken userToken: String, secretKey: String) async throws {
        if accountInfo == nil {
            // TODO: add supports for switching accounts.
            accountInfo = .init(userToken: userToken, secretKey: secretKey)
        }
        
        try await ensureConnection()
    }
}

private extension SessionManager {
    
    private func sendCommand(_ command: OutgoingMessage) {
        assert(isConnected)
        let data = try! JSONEncoder().encode(command)
        connection.sendData(data)
    }
}

// MARK: - Initialization
private extension SessionManager {
    
    func observeConnectionState() {
        connection
            .statePublisher
            .sink { [unowned self] state in
                handleConnectionStateChange(to: state)
            }
            .store(in: &cancellables)
        
        connection
            .dataPublisher
            .sink { [unowned self] data in
                logger.debug("Received data (\(data.count) bytes)")
                
                stateMachine.handleEvent(.dataReceived(data))
            }
            .store(in: &cancellables)
    }
    
    func prepareAndStartStateMachine() {
        idleState.onEnter = { [unowned self] in
            handleConnectionIdle()
        }
        
        signingInState.onEnter = { [unowned self] in
            doSignIn()
        }
        
        signedInState.onEnter = { [unowned self] in
            handleSignedIn()
        }
        
        closingState.onEnter = { [unowned self] in
            // TODO: implement this.
            _ = self
            fatalError("Not implemented")
        }
        
        stateMachine.addTransition(from: idleState, to: idleState, when: .disconnected)
        stateMachine.addTransition(from: idleState, to: connectingState, when: .connecting)
        stateMachine.addTransition(from: connectingState, to: idleState, when: .disconnected)
        stateMachine.addTransition(from: connectingState, to: signingInState, when: .connected)
        stateMachine.addTransition(from: signingInState, to: idleState, when: .disconnected)
        stateMachine.addTransition(from: signingInState, when: .dataReceived) { [unowned self] event in
            guard case let .dataReceived(data) = event else {
                fatalError("Unexpected event")
            }
            return handleSigningInData(data)
        }
        stateMachine.addTransition(from: signedInState, when: .dataReceived) { [unowned self] event in
            guard case let .dataReceived(data) = event else {
                fatalError("Unexpected event")
            }
            return handleData(data)
        }
        stateMachine.addTransition(from: closingState, to: idleState, when: .disconnected)
        
        stateMachine.start(with: idleState)
    }
}

// MARK: - Connection Lifecycle
private extension SessionManager {
    
    private func ensureConnection() async throws {
        if stateMachine.currentState == idleState {
            logger.info("Try starting the connection")
            
            assert(connectedFuture == nil)
            connectedFuture = .init()
            
            connection.start()
        }
        
        if let connectedFuture {
            try await connectedFuture.value
        }
    }
    
    private func handleConnectionStateChange(to newState: Connection.State) {
        logger.info("Connection state changed: \(String(describing: newState))")
        
        switch newState {
            case .idle:
                isConnected = false
                stateMachine.handleEvent(.disconnected)
            case .connecting:
                stateMachine.handleEvent(.connecting)
            case .connected:
                isConnected = true
                stateMachine.handleEvent(.connected)
        }
    }
    
    private func handleConnectionIdle() {
        struct _ConnectionError: Error { }
        
        if let connectedFuture {
            connectedFuture.reject(_ConnectionError())
            self.connectedFuture = nil
        }
    }
    
    private func doSignIn() {
        assert(stateMachine.currentState == signingInState)
        
        guard let accountInfo else {
            fatalError("Internal state is inconsistent")
        }
        
        // TODO: generate the device token.
        let loginCommand = LoginCommand(userToken: accountInfo.userToken,
                                        deviceToken: "",
                                        secretKey: accountInfo.secretKey)
        sendCommand(.login(loginCommand))
    }
    
    private func decodeIncomingMessage(from data: Data) -> Result<IncomingMessage, any Error> {
        do {
            let message = try JSONDecoder().decode(IncomingMessage.self, from: data)
            return .success(message)
        } catch {
            logger.error("Failed to decode incoming message: \(error)")
            return .failure(error)
        }
    }
    
    private func handleSigningInData(_ data: Data) -> ConnectionStateMachine.State {
        guard let message = try? decodeIncomingMessage(from: data).get() else {
            return closingState
        }
        
        switch message {
        case .loggedIn:
            return signedInState
        default:
            logger.error("\(#function): Unexpected message type at this stage")
            return closingState
        }
    }
    
    private func handleSignedIn() {
        guard let connectedFuture else {
            fatalError("Internal state is inconsistent")
        }
        connectedFuture.resolve(())
        
        syncManager.performSync()
    }
    
    private func handleData(_ data: Data) -> ConnectionStateMachine.State {
        guard let message = try? decodeIncomingMessage(from: data).get() else {
            return closingState
        }
        
        switch message {
        case .syncUpdates(let syncUpdates):
            syncManager.handleSyncResult(syncUpdates)
        default:
            logger.error("\(#function): Unexpected message type at this stage")
            return closingState
        }
        
        return signedInState
    }
}

// MARK: - Updates Syncing
extension SessionManager: SyncCoordinator {
    
    nonisolated func requestToSync(from pts: UInt64) {
        Task {
            await _requestToSync(from: pts)
        }
    }
    
    private func _requestToSync(from pts: UInt64) {
        guard stateMachine.currentState == signedInState else {
            logger.warning("Currently not connected, abort the sync operation")
            return
        }
        
        let syncCommand = SyncCommand(devicePts: pts)
        sendCommand(.sync(syncCommand))
    }
}
