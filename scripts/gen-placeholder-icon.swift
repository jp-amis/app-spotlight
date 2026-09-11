import AppKit

// Renders a 1024x1024 rounded-square gradient icon with an "S" glyph → out.png
let size = 1024.0
let img = NSImage(size: NSSize(width: size, height: size))
img.lockFocus()
let ctx = NSGraphicsContext.current!.cgContext

let rect = NSRect(x: 0, y: 0, width: size, height: size)
let path = NSBezierPath(roundedRect: rect, xRadius: size * 0.22, yRadius: size * 0.22)
path.addClip()

let colors = [NSColor(calibratedRed: 0.36, green: 0.20, blue: 0.90, alpha: 1).cgColor,
              NSColor(calibratedRed: 0.15, green: 0.55, blue: 0.98, alpha: 1).cgColor] as CFArray
let grad = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors,
                      locations: [0, 1])!
ctx.drawLinearGradient(grad, start: CGPoint(x: 0, y: size), end: CGPoint(x: size, y: 0), options: [])

let para = NSMutableParagraphStyle(); para.alignment = .center
let attrs: [NSAttributedString.Key: Any] = [
    .font: NSFont.systemFont(ofSize: size * 0.62, weight: .bold),
    .foregroundColor: NSColor.white,
    .paragraphStyle: para,
]
let s = NSAttributedString(string: "S", attributes: attrs)
let bs = s.size()
s.draw(at: NSPoint(x: (size - bs.width) / 2, y: (size - bs.height) / 2))

img.unlockFocus()

let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "out.png"
guard let tiff = img.tiffRepresentation,
      let rep = NSBitmapImageRep(data: tiff),
      let png = rep.representation(using: .png, properties: [:]) else {
    FileHandle.standardError.write("failed to render\n".data(using: .utf8)!); exit(1)
}
try! png.write(to: URL(fileURLWithPath: out))
print("wrote \(out)")
