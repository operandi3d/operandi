// Draws Operandi's scene-file (.opscene) document icon: a dark page with a
// folded corner, the capital-O logo from the app-icon master, and the
// extension as a label. The page is dark because the O's cream half vanishes
// on a white page; dark also matches the black Dock tile.
//
//   swift tools/document-icon/make-document-icon.swift \
//       assets/branding/operandi-app-icon.png \
//       assets/branding/operandi-document-icon.png
//
// Output is a 1024 x 1024 RGBA PNG. scripts/macos-app-bundle.sh turns it
// into OperandiScene.icns. Update branding-manifest.json (OP-DOC-1) after
// regenerating.
import AppKit

let args = CommandLine.arguments
guard args.count == 3, let logo = NSImage(contentsOfFile: args[1]) else {
    FileHandle.standardError.write("usage: make-document-icon.swift <app-icon.png> <out.png>\n".data(using: .utf8)!)
    exit(1)
}

let size = 1024
guard let rep = NSBitmapImageRep(
    bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)
else { exit(1) }
rep.size = NSSize(width: size, height: size)
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
NSGraphicsContext.current?.imageInterpolation = .high

func rgb(_ r: CGFloat, _ g: CGFloat, _ b: CGFloat, _ a: CGFloat = 1) -> NSColor {
    NSColor(srgbRed: r / 255, green: g / 255, blue: b / 255, alpha: a)
}

// Page: the usual macOS document proportions, y up from the bottom.
let left: CGFloat = 176, right: CGFloat = 848, bottom: CGFloat = 64, top: CGFloat = 960
let radius: CGFloat = 44, fold: CGFloat = 196

let page = NSBezierPath()
page.move(to: NSPoint(x: left + radius, y: bottom))
page.line(to: NSPoint(x: right - radius, y: bottom))
page.appendArc(withCenter: NSPoint(x: right - radius, y: bottom + radius), radius: radius, startAngle: 270, endAngle: 360)
page.line(to: NSPoint(x: right, y: top - fold))
page.line(to: NSPoint(x: right - fold, y: top))
page.line(to: NSPoint(x: left + radius, y: top))
page.appendArc(withCenter: NSPoint(x: left + radius, y: top - radius), radius: radius, startAngle: 90, endAngle: 180)
page.line(to: NSPoint(x: left, y: bottom + radius))
page.appendArc(withCenter: NSPoint(x: left + radius, y: bottom + radius), radius: radius, startAngle: 180, endAngle: 270)
page.close()

// Soft drop shadow under the page.
NSGraphicsContext.saveGraphicsState()
let shadow = NSShadow()
shadow.shadowColor = rgb(0, 0, 0, 0.45)
shadow.shadowOffset = NSSize(width: 0, height: -10)
shadow.shadowBlurRadius = 24
shadow.set()
rgb(30, 30, 33).setFill()
page.fill()
NSGraphicsContext.restoreGraphicsState()

NSGradient(starting: rgb(46, 46, 50), ending: rgb(24, 24, 27))!.draw(in: page, angle: -90)
rgb(255, 255, 255, 0.14).setStroke()
page.lineWidth = 4
page.stroke()

// The folded corner, lighter, with a shadow onto the page.
let flap = NSBezierPath()
flap.move(to: NSPoint(x: right - fold, y: top))
flap.line(to: NSPoint(x: right - fold + 18, y: top - fold + 18))
flap.curve(to: NSPoint(x: right - fold + 44, y: top - fold),
           controlPoint1: NSPoint(x: right - fold + 22, y: top - fold + 6),
           controlPoint2: NSPoint(x: right - fold + 30, y: top - fold))
flap.line(to: NSPoint(x: right, y: top - fold))
flap.close()
NSGraphicsContext.saveGraphicsState()
let flapShadow = NSShadow()
flapShadow.shadowColor = rgb(0, 0, 0, 0.5)
flapShadow.shadowOffset = NSSize(width: -6, height: -6)
flapShadow.shadowBlurRadius = 16
flapShadow.set()
rgb(78, 78, 84).setFill()
flap.fill()
NSGraphicsContext.restoreGraphicsState()
NSGradient(starting: rgb(92, 92, 98), ending: rgb(64, 64, 70))!.draw(in: flap, angle: 45)

// The O, centred above the label. The master is square with the O off
// centre (its visible bounds are about x 250..1010, y 150..1090 of 1254), so
// place it by those bounds.
let master = CGFloat(1254)
let oBounds = NSRect(x: 250, y: master - 1090, width: 760, height: 940)
let oHeight: CGFloat = 560
let scale = oHeight / oBounds.height
let oWidth = oBounds.width * scale
let oCenterX = (left + right) / 2, oCenterY: CGFloat = 560
let drawn = NSRect(
    x: oCenterX - oWidth / 2 - oBounds.minX * scale,
    y: oCenterY - oHeight / 2 - oBounds.minY * scale,
    width: master * scale, height: master * scale)
logo.draw(in: drawn, from: .zero, operation: .sourceOver, fraction: 1)

// Label in the wordmark cream.
let label = "OPSCENE" as NSString
let font = NSFont.systemFont(ofSize: 92, weight: .bold)
let attrs: [NSAttributedString.Key: Any] = [
    .font: font,
    .foregroundColor: rgb(242, 231, 216),
    .kern: 6,
]
let labelSize = label.size(withAttributes: attrs)
label.draw(at: NSPoint(x: oCenterX - labelSize.width / 2, y: 128), withAttributes: attrs)

NSGraphicsContext.restoreGraphicsState()
guard let png = rep.representation(using: .png, properties: [:]) else { exit(1) }
do {
    try png.write(to: URL(fileURLWithPath: args[2]))
} catch {
    FileHandle.standardError.write("\(error)\n".data(using: .utf8)!)
    exit(1)
}
