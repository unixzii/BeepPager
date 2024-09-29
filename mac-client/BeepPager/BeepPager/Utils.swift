//
//  Created by Cyandev on 2024/9/29.
//  Copyright (c) 2024 Cyandev. All rights reserved.
//

import Combine

struct UnsafeFutureSubject<Output, Failure: Error>: @unchecked Sendable {
    
    private typealias FutureType = Future<Output, Failure>
    
    private var future: FutureType
    private var promise: FutureType.Promise
    
    var value: Output {
        get async throws {
            try await future.value
        }
    }
    
    init() {
        var promise: FutureType.Promise!
        self.future = .init {
            promise = $0
        }
        self.promise = promise
    }
    
    func resolve(_ value: Output) {
        promise(.success(value))
    }
    
    func reject(_ error: Failure) {
        promise(.failure(error))
    }
}
