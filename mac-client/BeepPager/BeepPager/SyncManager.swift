//
//  Created by Cyandev on 2024/9/30.
//  Copyright (c) 2024 Cyandev. All rights reserved.
//

import os

protocol SyncCoordinator: AnyObject {
    
    func requestToSync(from pts: UInt64)
}

@MainActor
class SyncManager {
    
    private weak var syncCoordinator: (any SyncCoordinator)?
    private var localPts: UInt64 = 0
    
    private let logger: Logger = .init(subsystem: "me.cyandev.BeepPager.SyncManager", category: "general")
    
    init(syncCoordinator: any SyncCoordinator) {
        self.syncCoordinator = syncCoordinator
    }
    
    func performSync() {
        logger.info("Start syncing (localPts: \(self.localPts))")
        
        let syncCoordinator = ensureSyncCoordinator()
        syncCoordinator.requestToSync(from: localPts)
    }
    
    func handleSyncResult(_ syncUpdates: SyncUpdates) {
        // TODO: implement this.
        if !syncUpdates.isSynced {
            performSync()
        }
    }
    
    private func ensureSyncCoordinator() -> any SyncCoordinator {
        guard let syncCoordinator else {
            fatalError("SyncCoordinator has already been deallocated")
        }
        return syncCoordinator
    }
}
