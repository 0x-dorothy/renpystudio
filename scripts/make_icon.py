"""Generates the application icon set (RGB PNG + multi-size ICO) without external assets."""
from PIL import Image, ImageDraw
import os

SIZE = 1024
img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))

# Rounded square with a diagonal violet -> pink gradient.
grad = Image.new("RGB", (SIZE, SIZE))
gd = ImageDraw.Draw(grad)
c1 = (108, 76, 242)   # violet
c2 = (233, 106, 165)  # pink
for y in range(SIZE):
    for_x = y / (SIZE - 1)
    r = int(c1[0] + (c2[0] - c1[0]) * for_x)
    g = int(c1[1] + (c2[1] - c1[1]) * for_x)
    b = int(c1[2] + (c2[2] - c1[2]) * for_x)
    gd.line([(0, y), (SIZE, y)], fill=(r, g, b))

mask = Image.new("L", (SIZE, SIZE), 0)
md = ImageDraw.Draw(mask)
md.rounded_rectangle([0, 0, SIZE - 1, SIZE - 1], radius=int(SIZE * 0.22), fill=255)
img.paste(grad, (0, 0), mask)

d = ImageDraw.Draw(img)
# Dialogue "speech" bubble hint: three script lines.
line_color = (255, 255, 255, 235)
x0, x1 = int(SIZE * 0.24), int(SIZE * 0.76)
for i, w in enumerate([1.0, 0.72, 0.88]):
    y = int(SIZE * (0.28 + i * 0.115))
    d.rounded_rectangle([x0, y, x0 + int((x1 - x0) * w), y + int(SIZE * 0.045)],
                        radius=int(SIZE * 0.024), fill=line_color)
# Play triangle = "run the novel".
d.polygon([(int(SIZE * 0.42), int(SIZE * 0.63)),
           (int(SIZE * 0.42), int(SIZE * 0.85)),
           (int(SIZE * 0.68), int(SIZE * 0.74))], fill=(255, 255, 255, 245))

out = os.path.join(os.path.dirname(__file__), "icons")
os.makedirs(out, exist_ok=True)
img.save(os.path.join(out, "icon.png"))
for s in (32, 128, 256, 512):
    img.resize((s, s), Image.LANCZOS).save(os.path.join(out, f"{s}x{s}.png"))
img.resize((256, 256), Image.LANCZOS).save(
    os.path.join(out, "icon.ico"),
    sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
)
print("icons written to", out)
