//
//  ContentView.swift
//  SleepLight
//
//  Created by Clark McCauley on 2/24/26.
//

import SwiftUI

struct ContentView: View {
    @Environment(BLEManager.self) private var ble

    @State private var selectedColor: UIColor = .white
    @State private var brightness: Double = 10
    @State private var colorDebounceTask: Task<Void, Never>?

    private var swiftUIColor: Color {
        Color(selectedColor)
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                statusCard
                pickerCard
                brightnessCard
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 20)
        }
        .navigationTitle("SleepLight")
        .background(Color(.systemGroupedBackground))
        .onChange(of: selectedColor) {
            // FlexColorPicker has no touch-up event, so debounce: cancel any
            // pending send and schedule a new one after a short idle period.
            colorDebounceTask?.cancel()
            colorDebounceTask = Task {
                try? await Task.sleep(for: .milliseconds(150))
                guard !Task.isCancelled else { return }
                sendCurrentState()
            }
        }
    }

    // MARK: - Status

    private var statusCard: some View {
        HStack(spacing: 12) {
            Image(systemName: ble.isConnected ? "light.beacon.max" : "light.beacon.max.slash")
                .foregroundStyle(ble.isConnected ? .green : .secondary)
                .imageScale(.large)

            Text(statusText)
                .font(.subheadline)
                .foregroundStyle(ble.isConnected ? .primary : .secondary)

            Spacer()

            if let level = ble.batteryLevel {
                batteryView(level: level)
            }
        }
        .padding(16)
        .background(Color(.secondarySystemGroupedBackground))
        .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    private var statusText: String {
        if ble.isConnected { return "Connected" }
        if ble.isScanning  { return "Scanning…" }
        return "Disconnected"
    }

    private func batteryView(level: UInt8) -> some View {
        HStack(spacing: 4) {
            Image(systemName: batterySystemImage(for: level))
            Text("\(level)%")
                .font(.subheadline.monospacedDigit())
        }
        .foregroundStyle(.secondary)
    }

    private func batterySystemImage(for level: UInt8) -> String {
        switch level {
        case 0..<13:  return "battery.0percent"
        case 13..<38: return "battery.25percent"
        case 38..<63: return "battery.50percent"
        case 63..<88: return "battery.75percent"
        default:      return "battery.100percent"
        }
    }

    // MARK: - Color picker

    private var pickerCard: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("RGB Color")
                .font(.headline)
                .foregroundStyle(.secondary)

            FlexRGBColorPickerView(selectedColor: $selectedColor)
                .frame(height: 248)
        }
        .padding(20)
        .background(Color(.secondarySystemGroupedBackground))
        .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    // MARK: - Brightness

    private var brightnessCard: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Brightness")
                    .font(.headline)
                    .foregroundStyle(.secondary)
                Spacer()
                Text("\(Int(brightness))")
                    .font(.subheadline.monospacedDigit())
                    .foregroundStyle(.secondary)
            }
            // Firmware clamps brightness to 50; the slider range mirrors that cap.
            // onEditingChanged fires false exactly on touch-up, so no debounce needed.
            Slider(value: $brightness, in: 0...50, step: 1) { editing in
                if !editing { sendCurrentState() }
            }
            .tint(.yellow)
        }
        .padding(20)
        .background(Color(.secondarySystemGroupedBackground))
        .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
    }

    // MARK: - BLE

    private func sendCurrentState() {
        guard ble.isConnected else { return }
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        selectedColor.getRed(&r, green: &g, blue: &b, alpha: &a)
        ble.send(
            r: UInt8(r * 255),
            g: UInt8(g * 255),
            b: UInt8(b * 255),
            brightness: UInt8(brightness)
        )
    }
}

// MARK: - UIColor RGB components helper

private extension UIColor {
    /// Stable array representation used as the `Equatable` value for SwiftUI animation.
    var rgbComponents: [CGFloat] {
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        getRed(&r, green: &g, blue: &b, alpha: &a)
        return [r, g, b]
    }
}

#Preview {
    NavigationStack {
        ContentView()
    }
    .environment(BLEManager())
    .preferredColorScheme(.dark)
}
