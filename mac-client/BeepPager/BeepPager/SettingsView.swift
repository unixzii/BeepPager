//
//  Created by Cyandev on 2024/9/28.
//  Copyright (c) 2024 Cyandev. All rights reserved.
// 

import SwiftUI

struct SettingsView: View {
    
    var body: some View {
        Form {
            Section(header: Text("Account")) {
                AccountSettingsGroup()
            }
        }
        .formStyle(.grouped)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct AccountSettingsGroup: View {
    
    @State private var userToken: String = ""
    @State private var secretKey: String = ""
    @State private var isConnecting: Bool = false
    
    var body: some View {
        Group {
            TextField("User Token:", text: $userToken)
                .textContentType(.username)
            
            SecureField("Secret Key:", text: $secretKey)
                .textContentType(.password)
            
            HStack(spacing: 8) {
                Spacer()
                if isConnecting {
                    ProgressView()
                        .progressViewStyle(.circular)
                        .controlSize(.small)
                }
                Button("Sign In") {
                    isConnecting = true
                    
                    Task {
                        let sessionManager = SessionManager.default
                        try? await sessionManager.signIn(withUserToken: userToken, secretKey: secretKey)
                        isConnecting = false
                    }
                }
                .disabled(isConnecting || userToken.isEmpty || secretKey.isEmpty)
            }
        }
    }
}
