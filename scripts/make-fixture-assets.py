"""Creates placeholder art and audio for the end-to-end fixture project."""
import os
import shutil
import sys
from PIL import Image, ImageDraw

project = sys.argv[1] if len(sys.argv) > 1 else "."
sdk = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(__file__), "..", "..")
game = os.path.join(project, "game")

def gradient(path, size, top, bottom, label):
    img = Image.new("RGB", size)
    d = ImageDraw.Draw(img)
    for y in range(size[1]):
        t = y / max(1, size[1] - 1)
        color = tuple(int(top[i] + (bottom[i] - top[i]) * t) for i in range(3))
        d.line([(0, y), (size[0], y)], fill=color)
    d.rectangle([0, size[1] - 140, size[0], size[1]], fill=(0, 0, 0))
    d.text((40, 40), label, fill=(255, 255, 255))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    img.save(path)

def sprite(path, color):
    img = Image.new("RGBA", (360, 640), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([60, 40, 300, 620], radius=90, fill=color)
    d.ellipse([120, 90, 240, 210], fill=(255, 255, 255, 235))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    img.save(path)

gradient(os.path.join(game, "images/bg/room.png"), (1280, 720), (28, 26, 58), (8, 8, 16), "ROOM")
gradient(os.path.join(game, "images/bg/street.png"), (1280, 720), (18, 40, 70), (4, 8, 18), "STREET")
gradient(os.path.join(game, "images/bg/menu.png"), (1280, 720), (54, 32, 96), (6, 6, 14), "MENU")
sprite(os.path.join(game, "images/sprites/mira_happy.png"), (233, 106, 165, 235))
sprite(os.path.join(game, "images/sprites/mira_sad.png"), (150, 120, 210, 235))

# A real OGG file from the SDK so lint/playback see valid audio.
source = os.path.join(sdk, "the_question", "game", "illurock.opus")
if os.path.isfile(source):
    for rel in ["audio/theme.opus", "audio/menu.opus", "audio/click.opus", "audio/voice/mira_001.opus"]:
        target = os.path.join(game, rel)
        os.makedirs(os.path.dirname(target), exist_ok=True)
        shutil.copyfile(source, target)
    print("audio copied")
else:
    print("audio source missing:", source)

print("fixture assets ready")
