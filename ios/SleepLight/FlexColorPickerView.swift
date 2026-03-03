//
//  FlexColorPickerView.swift
//  SleepLight
//
//  Created by Clark McCauley on 2/24/26.
//

import SwiftUI
import FlexColorPicker

/// Bridges FlexColorPicker's UIKit RGB controls into a SwiftUI view.
///
/// Composes `RedSliderControl`, `GreenSliderControl`, `BlueSliderControl`,
/// and `ColorPreviewWithHex` managed by a single `ColorPickerController`.
struct FlexRGBColorPickerView: UIViewControllerRepresentable {
    @Binding var selectedColor: UIColor

    func makeCoordinator() -> Coordinator {
        Coordinator(selectedColor: $selectedColor)
    }

    func makeUIViewController(context: Context) -> RGBPickerViewController {
        let vc = RGBPickerViewController()
        vc.delegate = context.coordinator
        vc.selectedColor = selectedColor
        return vc
    }

    func updateUIViewController(_ uiViewController: RGBPickerViewController, context: Context) {
        uiViewController.selectedColor = selectedColor
    }

    // MARK: - Coordinator

    final class Coordinator: NSObject, ColorPickerDelegate {
        @Binding var selectedColor: UIColor

        init(selectedColor: Binding<UIColor>) {
            _selectedColor = selectedColor
        }

        func colorPicker(
            _ colorPicker: ColorPickerController,
            selectedColor: UIColor,
            usingControl: ColorControl
        ) {
            self.selectedColor = selectedColor
        }
    }
}

// MARK: - RGBPickerViewController

/// Lays out the FlexColorPicker RGB controls with autolayout.
///
/// Separated into its own UIViewController so intrinsic sizing and
/// lifecycle management are handled cleanly by UIKit.
final class RGBPickerViewController: UIViewController {
    var delegate: ColorPickerDelegate? {
        didSet { colorController.delegate = delegate }
    }

    var selectedColor: UIColor {
        get { colorController.selectedColor }
        set { colorController.selectedColor = newValue }
    }

    private let colorController = ColorPickerController()
    private let previewControl = ColorPreviewWithHex()
    private let redSlider = RedSliderControl()
    private let greenSlider = GreenSliderControl()
    private let blueSlider = BlueSliderControl()

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .clear
        setupSubviews()
        registerControls()
    }

    private func setupSubviews() {
        let stackView = UIStackView(arrangedSubviews: [
            previewControl,
            redSlider,
            greenSlider,
            blueSlider,
        ])
        stackView.axis = .vertical
        stackView.spacing = 16
        stackView.alignment = .fill
        stackView.translatesAutoresizingMaskIntoConstraints = false

        view.addSubview(stackView)

        NSLayoutConstraint.activate([
            stackView.topAnchor.constraint(equalTo: view.topAnchor),
            stackView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            stackView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            stackView.bottomAnchor.constraint(equalTo: view.bottomAnchor),

            previewControl.heightAnchor.constraint(equalToConstant: 80),
            redSlider.heightAnchor.constraint(equalToConstant: 44),
            greenSlider.heightAnchor.constraint(equalToConstant: 44),
            blueSlider.heightAnchor.constraint(equalToConstant: 44),
        ])

        previewControl.backgroundColor = .clear
        [redSlider, greenSlider, blueSlider].forEach { $0.backgroundColor = .clear }
    }

    private func registerControls() {
        colorController.addControl(previewControl)
        colorController.addControl(redSlider)
        colorController.addControl(greenSlider)
        colorController.addControl(blueSlider)
    }
}
