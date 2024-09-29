//
//  Created by Cyandev on 2024/9/29.
//  Copyright (c) 2024 Cyandev. All rights reserved.
//

import Foundation

fileprivate struct _StringKey: CodingKey {
    
    var stringValue: String
    var intValue: Int?
    
    init(stringValue: String) {
        self.stringValue = stringValue
    }
    
    init?(intValue: Int) {
        return nil
    }
}

enum OutgoingMessage: Codable {
    case login(LoginCommand)
    
    func encode(to encoder: any Encoder) throws {
        var commandType: String
        switch self {
            case .login(let loginCommand):
                try loginCommand.encode(to: encoder)
                commandType = type(of: loginCommand).type
        }
        
        var container = encoder.container(keyedBy: _StringKey.self)
        try container.encode(commandType, forKey: .init(stringValue: "cmd"))
    }
}

protocol Command {
    
    static var type: String { get }
}

struct LoginCommand: Codable, Command {
    
    static let type: String = "login"
    
    var userToken: String
    var deviceToken: String
    var secretKey: String
    
    enum CodingKeys: String, CodingKey {
        case userToken = "user_token"
        case deviceToken = "device_token"
        case secretKey = "secret_key"
    }
}

enum IncomingMessage: Decodable {
    case loggedIn
    
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: _StringKey.self)
        let eventType = try container.decode(String.self, forKey: .init(stringValue: "event"))
        
        struct _UnknownEventType: Error { }
        
        switch eventType {
        case "logged_in":
            self = Self.loggedIn
        default:
            throw _UnknownEventType()
        }
    }
}
