//
//  Created by Cyandev on 2024/9/29.
//  Copyright (c) 2024 Cyandev. All rights reserved.
//

protocol StateMachineEvent {
    
    associatedtype EventType: Hashable
    
    var eventType: EventType { get }
}

class StateMachine<Event> where Event: StateMachineEvent {
    
    typealias TransitionDecision = (Event) -> State
    
    class State: Equatable, Hashable {
        
        let debugLabel: String
        
        var onEnter: (() -> Void)?
        var onReenter: (() -> Void)?
        var onExit: (() -> Void)?
        
        fileprivate var transitions: [Event.EventType: TransitionDecision] = [:]
        
        static func == (lhs: StateMachine.State, rhs: StateMachine.State) -> Bool {
            lhs === rhs
        }
        
        init(debugLabel: String) {
            self.debugLabel = debugLabel
        }
        
        func hash(into hasher: inout Hasher) {
            hasher.combine(ObjectIdentifier(self))
        }
    }
    
    private var stateSet: Set<State> = .init()
    
    private(set) var currentState: State?
    var isStarted: Bool {
        return currentState != nil
    }
    
    func addState(with debugLabel: String = "") -> State {
        let state = State(debugLabel: debugLabel)
        stateSet.insert(state)
        return state
    }
    
    func addTransition(from fromState: State, to toState: State, when event: Event.EventType) {
        ensureOwnState(toState)
        addTransition(from: fromState, when: event) { _ in toState }
    }
    
    func addTransition(from fromState: State, when event: Event.EventType, decision: @escaping TransitionDecision) {
        ensureOwnState(fromState)
        fromState.transitions[event] = decision
    }
    
    func start(with initialState: State) {
        ensureOwnState(initialState)
        moveToState(initialState)
    }
    
    func handleEvent(_ event: Event) {
        guard let currentState else {
            fatalError("State machine is not started")
        }
        
        guard let decision = currentState.transitions[event.eventType] else {
            fatalError("Unknown transition from state \"\(currentState.debugLabel)\" with event \(event)")
        }
        let targetState = decision(event)
        ensureOwnState(targetState)
        moveToState(targetState)
    }
    
    private func ensureOwnState(_ state: State) {
        guard stateSet.contains(state) else {
            fatalError("State \"\(state.debugLabel)\" does not belong to this state machine")
        }
    }
    
    private func moveToState(_ state: State) {
        if currentState === state {
            state.onReenter?()
            return
        }
        
        let lastState = currentState
        currentState = state
        lastState?.onExit?()
        state.onEnter?()
    }
}
