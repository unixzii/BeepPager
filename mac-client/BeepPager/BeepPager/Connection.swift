//
//  Created by Cyandev on 2024/9/28.
//  Copyright (c) 2024 Cyandev. All rights reserved.
// 

import Foundation

@MainActor
final class Connection: NSObject {
    
    private enum State {
        case idle
        case connecting
        case connected
    }
    
    private let urlSession: URLSession
    private var currentTask: URLSessionWebSocketTask?
    private var state: State = .idle
    
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
    
    private func scheduleReceiving() {
        guard let currentTask, state == .connected else {
            fatalError("Internal state is inconsistent")
        }
        
        currentTask.receive { [weak self] result in
            // TODO: handle the result
            guard let self else {
                return
            }
            
            Task { @MainActor in
                if state == .connected {
                    scheduleReceiving()
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
