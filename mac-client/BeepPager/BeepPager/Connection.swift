//
//  Created by Cyandev on 2024/9/28.
//  Copyright (c) 2024 Cyandev. All rights reserved.
// 

import Foundation
import Combine
import os

@MainActor
final class Connection: NSObject {
    
    enum State {
        case idle
        case connecting
        case connected
    }
    
    private let urlSession: URLSession
    private var currentTask: URLSessionWebSocketTask?
    private var state: State = .idle {
        willSet {
            if newValue == .idle {
                currentTask?.cancel()
                currentTask = nil
            }
        }
        didSet {
            stateSubject.send(state)
        }
    }
    
    private var stateSubject: CurrentValueSubject<State, Never> = .init(.idle)
    var statePublisher: AnyPublisher<State, Never> {
        stateSubject.eraseToAnyPublisher()
    }
    
    private var dataSubject: PassthroughSubject<Data, Never> = .init()
    var dataPublisher: AnyPublisher<Data, Never> {
        dataSubject.eraseToAnyPublisher()
    }
    
    private let logger: Logger = .init(subsystem: "me.cyandev.BeepPager.Connection", category: "general")
    
    override init() {
        let urlSessionConfiguration = URLSessionConfiguration.ephemeral
        urlSession = .init(configuration: urlSessionConfiguration, delegate: nil, delegateQueue: .main)
    }
    
    func start() {
        guard state == .idle else {
            return
        }
        
        state = .connecting
        
        let url = URL(string: "ws://127.0.0.1:5020/ws")!
        let task = urlSession.webSocketTask(with: url)
        task.delegate = self
        task.resume()
        
        currentTask = task
    }
    
    func sendData(_ data: Data) {
        guard state == .connected else {
            return
        }
        
        guard let currentTask else {
            fatalError("Internal state is inconsistent")
        }
        currentTask.send(.data(data)) { [weak self] error in
            guard let self else {
                return
            }
            
            if let error {
                logger.error("Failed to send data: \(error)")
                Task { @MainActor in
                    self.state = .idle
                }
            }
        }
    }
    
    private func scheduleReceiving() {
        guard let currentTask, state == .connected else {
            fatalError("Internal state is inconsistent")
        }
        
        currentTask.receive { [weak self] result in
            guard let self else {
                return
            }
            
            Task { @MainActor in
                do {
                    let message = try result.get()
                    guard state == .connected else {
                        return
                    }
                    
                    switch message {
                    case .data(let data):
                        dataSubject.send(data)
                    case .string(let string):
                        dataSubject.send(string.data(using: .utf8) ?? .init())
                    @unknown default:
                        fatalError("Unknown message type: \(message)")
                    }
                    
                    scheduleReceiving()
                } catch {
                    state = .idle
                }
            }
        }
    }
}

extension Connection: URLSessionWebSocketDelegate {
    
    nonisolated func urlSession(_ session: URLSession,
                                webSocketTask: URLSessionWebSocketTask,
                                didOpenWithProtocol protocol: String?) {
        MainActor.assumeIsolated {
            state = .connected
            scheduleReceiving()
        }
    }
    
    nonisolated func urlSession(_ session: URLSession,
                                webSocketTask: URLSessionWebSocketTask,
                                didCloseWith closeCode: URLSessionWebSocketTask.CloseCode,
                                reason: Data?) {
        MainActor.assumeIsolated {
            state = .idle
        }
    }
    
    nonisolated func urlSession(_ session: URLSession,
                                task: URLSessionTask,
                                didCompleteWithError error: (any Error)?) {
        MainActor.assumeIsolated {
            state = .idle
        }
    }
}
