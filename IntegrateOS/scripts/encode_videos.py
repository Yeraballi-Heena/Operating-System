import os
import subprocess
import struct

FFMPEG_BIN = r"C:\Users\yheen\AppData\Local\Programs\Python\Python313\Lib\site-packages\imageio_ffmpeg\binaries\ffmpeg-win-x86_64-v9.0.2.exe"

def encode_video_to_ivid(mp4_path, out_ivid_path, target_w=120, target_h=68, target_fps=6, max_duration_sec=10):
    print(f"Encoding {mp4_path} -> {out_ivid_path} ({target_w}x{target_h} @ {target_fps}fps, {max_duration_sec}s)...")
    
    cmd = [
        FFMPEG_BIN,
        "-y",
        "-i", mp4_path,
        "-t", str(max_duration_sec),
        "-vf", f"fps={target_fps},scale={target_w}:{target_h}:flags=bicubic",
        "-f", "rawvideo",
        "-pix_fmt", "rgb24",
        "pipe:1"
    ]
    
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    raw_data, _ = proc.communicate()
    
    frame_size = target_w * target_h * 3
    num_frames = len(raw_data) // frame_size
    print(f"Extracted {num_frames} frames ({len(raw_data)} bytes raw)")
    
    encoded_frames = []
    
    for f_idx in range(num_frames):
        frame_bytes = raw_data[f_idx * frame_size : (f_idx + 1) * frame_size]
        pixels_565 = []
        for p in range(0, len(frame_bytes), 3):
            # Quantize 3 bits off each channel to filter sensor noise and enable clean RLE runs
            r = (frame_bytes[p] >> 3)
            g = (frame_bytes[p+1] >> 2)
            b = (frame_bytes[p+2] >> 3)
            
            # Subtle low-bit round to improve compression
            r_q = (r >> 1) << 1
            g_q = (g >> 1) << 1
            b_q = (b >> 1) << 1
            
            val565 = (r_q << 11) | (g_q << 5) | b_q
            pixels_565.append(val565)
            
        rle_chunks = bytearray()
        cur_val = pixels_565[0]
        cur_count = 1
        
        for p_idx in range(1, len(pixels_565)):
            val = pixels_565[p_idx]
            if val == cur_val and cur_count < 255:
                cur_count += 1
            else:
                rle_chunks.append(cur_count)
                rle_chunks.extend(struct.pack("<H", cur_val))
                cur_val = val
                cur_count = 1
        rle_chunks.append(cur_count)
        rle_chunks.extend(struct.pack("<H", cur_val))
        
        encoded_frames.append(bytes(rle_chunks))
        
    header = bytearray()
    header.extend(b"IVID")
    header.extend(struct.pack("<H", target_w))
    header.extend(struct.pack("<H", target_h))
    header.append(target_fps)
    header.extend(struct.pack("<H", num_frames))
    header.extend(b"\x00" * 5)
    assert len(header) == 16
    
    with open(out_ivid_path, "wb") as f:
        f.write(header)
        for frame in encoded_frames:
            f.write(struct.pack("<I", len(frame)))
            f.write(frame)
            
    out_size = os.path.getsize(out_ivid_path)
    print(f"[OK] Wrote {out_ivid_path}: {out_size} bytes ({num_frames} frames @ {target_fps} fps)")

if __name__ == "__main__":
    vid1 = r"D:\WhatsApp Video 2026-11-21 at 12.49.48 PM.mp4"
    vid2 = r"D:\WhatsApp Video 2026-11-21 at 12.53.37 PM.mp4"
    out_dir = r"d:\IntegrateOS\kernel\src\assets\videos"
    os.makedirs(out_dir, exist_ok=True)
    
    encode_video_to_ivid(vid1, os.path.join(out_dir, "clip_d1.vid"), target_w=120, target_h=68, target_fps=6, max_duration_sec=12)
    encode_video_to_ivid(vid2, os.path.join(out_dir, "clip_d2.vid"), target_w=120, target_h=68, target_fps=6, max_duration_sec=12)
