//
//  Created by Cyandev on 2024/9/28.
//  Copyright (c) 2024 Cyandev. All rights reserved.
// 

import Cocoa
import SwiftUI

@main
class AppDelegate: NSObject, NSApplicationDelegate {
    
    private var settingsWindow: NSWindow?
    
    func applicationDidFinishLaunching(_ aNotification: Notification) {
        // Insert code here to initialize your application
    }

    func applicationWillTerminate(_ aNotification: Notification) {
        // Insert code here to tear down your application
    }

    func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
        return true
    }

    @IBAction func showSettingsWindow(_ sender: Any) {
        if let settingsWindow {
            settingsWindow.makeKeyAndOrderFront(nil)
            return
        }
        
        let controller = NSHostingController(rootView: SettingsView())
        let settingsWindow = NSWindow(contentViewController: controller)
        settingsWindow.title = "Settings"
        settingsWindow.styleMask.remove([.miniaturizable, .resizable])
        settingsWindow.setContentSize(.init(width: 300, height: 200))
        settingsWindow.makeKeyAndOrderFront(nil)
        settingsWindow.center()
        self.settingsWindow = settingsWindow
    }
}

