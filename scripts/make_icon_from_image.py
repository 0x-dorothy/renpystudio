"""Builds the app icon set from the supplied artwork (white corners -> transparency)."""
from PIL import Image, ImageDraw
import os
import sys

source = sys.argv[1]
out = sys.argv[2]
size = 1024

img = Image.open(source).convert("RGBA")
# Center-crop to a square.
side = min(img.size)
left = (img.width - side) // 2
top = (img.height - side) // 2
img = img.crop((left, top, left + side, top + side)).resize((size, size), Image.LANCZOS)

# The artwork has a rounded body on white; keep everything inside a rounded mask.
mask = Image.new("L", (size, size), 0)
draw = ImageDraw.Draw(mask)
draw.rounded_rectangle([0, 0, size - 1, size - 1], radius=int(size * 0.225), fill=255)
img.putalpha(mask)

# Slightly trim the white halo at the very edge by clamping near-white pixels.
pixels = img.load()
for y in range(size):
    for x in range(size):
        r, g, b, a = pixels[x, y]
        if a and r > 248 and g > 248 and b > 248:
            pixels[x, y] = (r, g, b, 0)

os.makedirs(out, exist_ok=True)
img.save(os.path.join(out, "icon.png"))
for s in (32, 128, 256, 512):
    img.resize((s, s), Image.LANCZOS).save(os.path.join(out, f"{s}x{s}.png"))
img.resize((256, 256), Image.LANCZOS).save(
    os.path.join(out, "icon.ico"),
    sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
)
print("icons ->", out)
