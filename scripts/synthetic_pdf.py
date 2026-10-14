"""Small fictional text PDF generator for development smoke tests only."""
from pathlib import Path

def build_pdf(pages: list[list[str]]) -> bytes:
    objects: list[bytes] = [b"", b"", b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"]
    kids = []
    for lines in pages:
        stream = "BT /F1 12 Tf 20 TL 72 720 Td\n"
        for line in lines:
            escaped = line.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")
            stream += f"({escaped}) Tj T*\n"
        data = (stream + "ET\n").encode("ascii")
        stream_id = len(objects) + 1
        objects.append(f"<< /Length {len(data)} >>\nstream\n".encode() + data + b"endstream")
        page_id = len(objects) + 1
        objects.append(f"<< /Type /Page /Parent 2 0 R /Contents {stream_id} 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> >>".encode())
        kids.append(f"{page_id} 0 R")
    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>"
    objects[1] = f"<< /Type /Pages /Kids [{' '.join(kids)}] /Count {len(kids)} >>".encode()
    result = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for i, obj in enumerate(objects, 1):
        offsets.append(len(result))
        result.extend(f"{i} 0 obj\n".encode() + obj + b"\nendobj\n")
    xref = len(result)
    result.extend(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode())
    for offset in offsets[1:]:
        result.extend(f"{offset:010d} 00000 n \n".encode())
    result.extend(f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    return bytes(result)

if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", type=Path)
    options = parser.parse_args()
    options.path.write_bytes(build_pdf([["Synthetic checking statement", "2026-10-01 Fictional payroll 1000.01", "2026-10-02 Fictional groceries -25.11"]]))
