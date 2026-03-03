//
//  BLEManager.swift
//  SleepLight
//
//  Created by Clark McCauley on 2/24/26.
//

import CoreBluetooth
import Observation
import OSLog

private let logger = Logger(subsystem: "clarkmccauley.SleepLight", category: "BLE")

/// Manages BLE central role: scanning, connecting, and communicating with the SleepLight peripheral.
@MainActor
@Observable
final class BLEManager: NSObject {

    // MARK: - Observable state

    var isConnected = false
    var isScanning = false
    var batteryLevel: UInt8?

    // MARK: - Private

    /// `queue: nil` routes CoreBluetooth callbacks to the main queue so all state stays on the main actor.
    private var central: CBCentralManager!
    private var peripheral: CBPeripheral?
    private var stateCharacteristic: CBCharacteristic?
    private var batteryCharacteristic: CBCharacteristic?

    private static let lightServiceUUID = CBUUID(string: "f3e0c001-8b6f-4d2e-a2d0-6b9c3f2a0000")
    private static let stateCharUUID = CBUUID(string: "f3e0c002-8b6f-4d2e-a2d0-6b9c3f2a0000")
    private static let batteryServiceUUID = CBUUID(string: "180F")
    private static let batteryCharUUID = CBUUID(string: "2A19")

    override init() {
        super.init()
        central = CBCentralManager(delegate: self, queue: nil)
    }

    // MARK: - Public interface

    func startScan() {
        guard self.central.state == .poweredOn else {
            logger.warning("startScan called but central is not powered on (state: \(self.central.state.debugDescription))")
            return
        }
        logger.info("Scanning for peripherals advertising \(Self.lightServiceUUID)…")
        isScanning = true
        self.central.scanForPeripherals(
            withServices: [Self.lightServiceUUID],
            options: [CBCentralManagerScanOptionAllowDuplicatesKey: false]
        )
    }

    func stopScan() {
        logger.info("Scan stopped")
        central.stopScan()
        isScanning = false
    }

    /// Encodes RGB + brightness as [R, G, B, brightness] and writes to the
    /// state characteristic. The firmware's `from_ble_u32` reinterprets this
    /// as a little-endian UInt32, so the intuitive byte order is correct.
    func send(r: UInt8, g: UInt8, b: UInt8, brightness: UInt8) {
        guard let p = peripheral, let char = stateCharacteristic else {
            logger.debug("send skipped: not connected or state characteristic not ready")
            return
        }
        p.writeValue(Data([r, g, b, brightness]), for: char, type: .withResponse)
    }

    func disconnect() {
        guard let p = peripheral else { return }
        central.cancelPeripheralConnection(p)
    }
}

// MARK: - CBManagerState readable description

private extension CBManagerState {
    var debugDescription: String {
        switch self {
        case .unknown:      return "unknown"
        case .resetting:    return "resetting"
        case .unsupported:  return "unsupported"
        case .unauthorized: return "unauthorized"
        case .poweredOff:   return "poweredOff"
        case .poweredOn:    return "poweredOn"
        @unknown default:   return "unhandled(\(rawValue))"
        }
    }
}

// MARK: - CBCentralManagerDelegate

extension BLEManager: CBCentralManagerDelegate {

    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        logger.info("Central state → \(central.state.debugDescription)")
        if central.state == .poweredOn {
            startScan()
        }
    }

    func centralManager(
        _ central: CBCentralManager,
        didDiscover peripheral: CBPeripheral,
        advertisementData: [String: Any],
        rssi RSSI: NSNumber
    ) {
        let localName = advertisementData[CBAdvertisementDataLocalNameKey] as? String ?? peripheral.name ?? "<unnamed>"
        let services = (advertisementData[CBAdvertisementDataServiceUUIDsKey] as? [CBUUID])?.map(\.uuidString) ?? []
        logger.debug("Discovered: \"\(localName)\" | RSSI \(RSSI) | services \(services)")

        guard localName == "SleepLight" else { return }
        logger.info("Found SleepLight — connecting (RSSI \(RSSI))")
        self.peripheral = peripheral
        stopScan()
        central.connect(peripheral, options: nil)
    }

    func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
        logger.info("Connected to \(peripheral.name ?? peripheral.identifier.uuidString)")
        isConnected = true
        peripheral.delegate = self
        peripheral.discoverServices([Self.lightServiceUUID, Self.batteryServiceUUID])
    }

    func centralManager(
        _ central: CBCentralManager,
        didDisconnectPeripheral peripheral: CBPeripheral,
        error: Error?
    ) {
        if let error {
            logger.error("Disconnected with error: \(error.localizedDescription)")
        } else {
            logger.info("Disconnected cleanly — restarting scan")
        }
        isConnected = false
        self.peripheral = nil
        stateCharacteristic = nil
        batteryCharacteristic = nil
        startScan()
    }
}

// MARK: - CBPeripheralDelegate

extension BLEManager: CBPeripheralDelegate {

    func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        if let error {
            logger.error("Service discovery failed: \(error.localizedDescription)")
            return
        }
        let found = peripheral.services?.map(\.uuid.uuidString) ?? []
        logger.info("Discovered services: \(found)")
        peripheral.services?.forEach { service in
            switch service.uuid {
            case Self.lightServiceUUID:
                peripheral.discoverCharacteristics([Self.stateCharUUID], for: service)
            case Self.batteryServiceUUID:
                peripheral.discoverCharacteristics([Self.batteryCharUUID], for: service)
            default:
                logger.debug("Ignoring unexpected service \(service.uuid)")
            }
        }
    }

    func peripheral(
        _ peripheral: CBPeripheral,
        didDiscoverCharacteristicsFor service: CBService,
        error: Error?
    ) {
        if let error {
            logger.error("Characteristic discovery failed for \(service.uuid): \(error.localizedDescription)")
            return
        }
        let found = service.characteristics?.map(\.uuid.uuidString) ?? []
        logger.info("Discovered characteristics for \(service.uuid): \(found)")
        service.characteristics?.forEach { char in
            switch char.uuid {
            case Self.stateCharUUID:
                logger.debug("Subscribed to state characteristic")
                stateCharacteristic = char
                peripheral.setNotifyValue(true, for: char)
            case Self.batteryCharUUID:
                logger.debug("Subscribed to battery characteristic")
                batteryCharacteristic = char
                peripheral.readValue(for: char)
                peripheral.setNotifyValue(true, for: char)
            default:
                logger.debug("Ignoring unexpected characteristic \(char.uuid)")
            }
        }
    }

    func peripheral(
        _ peripheral: CBPeripheral,
        didUpdateValueFor characteristic: CBCharacteristic,
        error: Error?
    ) {
        if let error {
            logger.error("Value update error for \(characteristic.uuid): \(error.localizedDescription)")
            return
        }
        guard let data = characteristic.value else { return }
        if characteristic.uuid == Self.batteryCharUUID {
            batteryLevel = data.first
            logger.debug("Battery level: \(data.first.map { String($0) } ?? "nil")%")
        }
    }
}
