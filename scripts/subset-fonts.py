"""Build the OFL-licensed Latin/IPA fallback; see styles.css for the command."""

from io import BytesIO
from pathlib import Path
from urllib.request import urlopen

from fontTools import subset
from fontTools.ttLib import TTFont

# Pin the sources, including the licence, rather than following Google Fonts main.
SOURCE = (
    "https://raw.githubusercontent.com/google/fonts/"
    "c92f92dac919ca89a14d75655e89cf3f525310ca/ofl/gentiumbookplus/"
)
DEST = Path(__file__).resolve().parents[1] / "web/public/fonts"
# Latin, IPA and combining marks, Latin Extended Additional, then the
# punctuation and arrows of prose, so one face sets a whole sentence.
UNICODES = (
    list(range(0x20, 0x370))
    + list(range(0x1E00, 0x1F00))
    + list(range(0x2000, 0x2070))
    + list(range(0x2190, 0x2200))
)

DEST.mkdir(parents=True, exist_ok=True)
for style in ("Regular", "Italic", "Bold", "BoldItalic"):
    with urlopen(f"{SOURCE}GentiumBookPlus-{style}.ttf") as response:
        font = TTFont(BytesIO(response.read()))
    options = subset.Options()
    options.flavor = "woff2"
    # Keep composition and attachment, not optional stylistic glyph variants.
    options.layout_features = ["ccmp", "locl", "mark", "mkmk", "kern", "liga"]
    options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14, 16, 17]
    options.name_legacy = True
    options.name_languages = ["*"]
    subsetter = subset.Subsetter(options=options)
    subsetter.populate(unicodes=UNICODES)
    subsetter.subset(font)
    # Subsetting is a modification: honour the reserved names in the OFL.
    names = {
        1: "Umran Latin", 2: style, 3: f"UmranLatin-{style}",
        4: f"Umran Latin {style}", 6: f"UmranLatin-{style}",
        16: "Umran Latin", 17: style,
    }
    for record in font["name"].names:
        if record.nameID in names:
            record.string = names[record.nameID].encode(record.getEncoding())
    font.flavor = "woff2"
    font.save(DEST / f"umran-latin-{style.lower()}.woff2")

with urlopen(f"{SOURCE}OFL.txt") as response:
    (DEST / "OFL.txt").write_bytes(response.read())
