//
//  SleepLightApp.swift
//  SleepLight
//
//  Created by Clark McCauley on 2/24/26.
//

import SwiftUI

@main
struct SleepLightApp: App {
    @State private var ble = BLEManager()

    var body: some Scene {
        WindowGroup {
            NavigationStack {
                ContentView()
            }
            .environment(ble)
        }
    }
}
