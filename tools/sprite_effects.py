#!/usr/bin/env python3
import os
import sys
import math
import random
import argparse
import numpy as np
import cv2

class Particle:
    def __init__(self, x, y, vx, vy, lifetime, scale, rot, rot_speed, wobble_speed, wobble_amp):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy
        self.lifetime = lifetime
        self.scale = scale
        self.rot = rot
        self.rot_speed = rot_speed
        self.wobble_speed = wobble_speed
        self.wobble_amp = wobble_amp
        self.age = 0

def create_default_sprite(sprite_type="leaf"):
    """Generates a default transparent PNG-like sprite image using NumPy/OpenCV."""
    img = np.zeros((128, 128, 4), dtype=np.uint8)
    center = (64, 64)
    
    if sprite_type == "star":
        # Draw a beautiful glowing 4-point star / sparkle
        for y in range(128):
            for x in range(128):
                dx = abs(x - 64)
                dy = abs(y - 64)
                # Math formula for a flare shape
                dist = dx + dy
                if dist < 64:
                    flare = (1.0 - dist / 64.0) ** 3.0
                    glow = math.exp(-((dx*dx + dy*dy) / 400.0))
                    alpha = int(255 * max(flare, glow * 0.4))
                    img[y, x] = [180, 240, 255, alpha] # Warm whitish-blue sparkle
    elif sprite_type == "leaf":
        # Draw an organic orange-red autumn leaf
        mask = np.zeros((128, 128), dtype=np.uint8)
        # Leaf body ellipse
        cv2.ellipse(mask, center, (40, 18), 35, 0, 360, 255, -1)
        # Stem
        cv2.line(mask, (15, 25), (64, 64), 255, 3)
        # Veins (main vein)
        cv2.line(mask, (64, 64), (105, 95), 255, 2)
        
        # Color fill: Autumn Maple Red
        img[mask > 0] = [34, 76, 215, 240] # BGR format: R=215, G=76, B=34
    elif sprite_type == "cloud":
        # Draw a fluffy cloud using overlapping circles and soft Gaussian blur
        mask = np.zeros((128, 128), dtype=np.uint8)
        cv2.circle(mask, (64, 60), 25, 255, -1)
        cv2.circle(mask, (44, 68), 18, 255, -1)
        cv2.circle(mask, (84, 68), 20, 255, -1)
        cv2.circle(mask, (54, 76), 14, 255, -1)
        cv2.circle(mask, (74, 76), 14, 255, -1)
        
        # Gaussian Blur to make the cloud soft and fluffy
        blurred = cv2.GaussianBlur(mask, (15, 15), 0)
        for y in range(128):
            for x in range(128):
                alpha = blurred[y, x]
                if alpha > 0:
                    # Soft white cloud (B=255, G=255, R=255)
                    img[y, x] = [255, 255, 255, int(alpha * 0.85)]
    elif sprite_type == "rain":
        # Draw a vertical rain drop / streak
        mask = np.zeros((128, 128), dtype=np.uint8)
        cv2.line(mask, (64, 10), (64, 118), 255, 3)
        # Gaussian blur for soft rain streak
        blurred = cv2.GaussianBlur(mask, (5, 5), 0)
        for y in range(128):
            for x in range(128):
                alpha = blurred[y, x]
                if alpha > 0:
                    # Soft whitish-blue rain streak (B=235, G=215, R=180)
                    img[y, x] = [235, 215, 180, int(alpha * 0.6)]
    else:
        # Default circle
        cv2.circle(img, center, 32, (255, 255, 255, 255), -1)
        
    return img

def main():
    parser = argparse.ArgumentParser(description="OpenCut Universal Sprite Particle Effect Generator")
    parser.add_argument("--sprite", type=str, default="", help="Path to custom transparent PNG sprite image.")
    parser.add_argument("--sprite-type", type=str, default="leaf", choices=["leaf", "star", "circle", "cloud", "rain"], help="Default sprite type if no custom path is given.")
    parser.add_argument("--output", type=str, default="output_effect.mp4", help="Path to save the output MP4 video.")
    parser.add_argument("--duration", type=float, default=10.0, help="Video duration in seconds.")
    parser.add_argument("--fps", type=int, default=30, help="Frames per second.")
    parser.add_argument("--width", type=int, default=1920, help="Video width in pixels.")
    parser.add_argument("--height", type=int, default=1080, help="Video height in pixels.")
    
    # Physics settings
    parser.add_argument("--density", type=float, default=20.0, help="Average number of particles spawned per second.")
    parser.add_argument("--gravity", type=float, default=3.0, help="Y-axis gravity force (positive falls, negative floats up).")
    parser.add_argument("--wind", type=float, default=1.0, help="X-axis wind force.")
    parser.add_argument("--wobble", type=float, default=2.0, help="Amplitude of horizontal swaying/wobbling.")
    parser.add_argument("--y-min", type=float, default=0.0, help="Minimum Y spawn ratio (0.0 to 1.0).")
    parser.add_argument("--y-max", type=float, default=1.0, help="Maximum Y spawn ratio (0.0 to 1.0).")
    parser.add_argument("--scale-min", type=float, default=0.2, help="Minimum scale factor for particles.")
    parser.add_argument("--scale-max", type=float, default=0.6, help="Maximum scale factor for particles.")
    parser.add_argument("--rot-speed", type=float, default=3.0, help="Maximum rotation speed in degrees per frame.")
    parser.add_argument("--fade-in", type=float, default=0.4, help="Fade-in time for particles in seconds.")
    parser.add_argument("--fade-out", type=float, default=1.0, help="Fade-out time for particles in seconds.")
    
    args = parser.parse_args()

    # Load or generate sprite image
    if args.sprite and os.path.exists(args.sprite):
        print(f"Loading custom sprite from: {args.sprite}")
        sprite = cv2.imread(args.sprite, cv2.IMREAD_UNCHANGED)
        if sprite is None or sprite.shape[2] < 4:
            print("Error: Custom sprite must be a valid PNG image with an alpha channel (4 channels). Fallback to default.")
            sprite = create_default_sprite(args.sprite_type)
    else:
        print(f"Using default built-in sprite type: {args.sprite_type}")
        sprite = create_default_sprite(args.sprite_type)

    sh, sw = sprite.shape[:2]
    
    # Calculate output parameters
    total_frames = int(args.duration * args.fps)
    spawn_rate = args.density / args.fps
    
    # Convert per-second forces to per-frame forces
    # We apply small drag coefficient to keep terminal speed steady
    gravity_per_frame = args.gravity / args.fps
    wind_per_frame = args.wind / args.fps
    
    # Create output video writer
    # Use MP4V codec which is widely supported
    fourcc = cv2.VideoWriter_fourcc(*"mp4v")
    out = cv2.VideoWriter(args.output, fourcc, args.fps, (args.width, args.height))
    
    if not out.isOpened():
        print(f"Error: Could not open VideoWriter for {args.output}")
        sys.exit(1)

    particles = []
    
    def spawn_particle(full_screen=False):
        # Scale and lifecycle
        scale = random.uniform(args.scale_min, args.scale_max)
        lifetime = int(random.uniform(5.0, 9.0) * args.fps) # longer lifetime for slow drifting particles
        
        # Position spawning
        # Spawning margin to hide popping edges
        margin = int(max(sw, sh) * scale)
        
        y_min_val = args.y_min * args.height
        y_max_val = args.y_max * args.height
        
        if full_screen:
            x = random.uniform(-margin, args.width + margin)
            y = random.uniform(y_min_val - margin, y_max_val + margin)
        else:
            # If gravity is very small, we do horizontal movement and spawn at left/right edges
            if abs(args.gravity) < 0.5:
                spawn_left = random.choice([True, False]) if args.wind == 0 else (args.wind >= 0)
                if spawn_left:
                    x = float(-margin) # enters from left
                else:
                    x = float(args.width + margin) # enters from right
                y = random.uniform(y_min_val - margin, y_max_val + margin)
            else:
                x = random.uniform(-margin, args.width + margin)
                if args.gravity >= 0:
                    y = float(-margin) # Spawn at the top
                else:
                    y = float(args.height + margin) # Spawn at the bottom
                
        # Velocities
        if abs(args.gravity) < 0.5:
            # Horizontal motion
            if args.wind == 0:
                vx = random.uniform(0.3, 0.8) if (x == -margin) else random.uniform(-0.8, -0.3)
            else:
                vx = random.uniform(0.3, 0.8) if args.wind >= 0 else random.uniform(-0.8, -0.3)
            vy = random.uniform(-0.1, 0.1) # extremely small vertical drift
        else:
            vx = random.uniform(-1.0, 1.0)
            vy = random.uniform(1.0, 2.0) if args.gravity >= 0 else random.uniform(-2.0, -1.0)
        
        # Rotations and wobbling
        if args.sprite_type == "rain":
            # Align rain streaks to the fall velocity direction
            rot = math.degrees(math.atan2(vy, vx)) - 90
            rot_speed = 0.0
            wobble_speed = 0.0
            wobble_amp = 0.0
        else:
            rot = random.uniform(0, 360)
            rot_speed = random.uniform(-args.rot_speed, args.rot_speed)
            wobble_speed = random.uniform(0.05, 0.15)
            wobble_amp = random.uniform(0.1, args.wobble)
        
        p = Particle(x, y, vx, vy, lifetime, scale, rot, rot_speed, wobble_speed, wobble_amp)
        # If pre-populated, set a random starting age to distribute states
        if full_screen:
            p.age = random.randint(0, lifetime // 2)
        return p

    # Pre-populate screen with particles so it starts full
    initial_count = int(args.density * 3.0) # roughly 3 seconds worth of particles
    for _ in range(initial_count):
        particles.append(spawn_particle(full_screen=True))

    print(f"Generating {args.duration}s video ({total_frames} frames) at {args.width}x{args.height}...")

    for frame_idx in range(total_frames):
        # 1. Spawn new particles for this frame
        num_to_spawn = int(spawn_rate) + (1 if random.random() < (spawn_rate % 1.0) else 0)
        for _ in range(num_to_spawn):
            particles.append(spawn_particle(full_screen=False))
            
        # Create a black frame background
        frame = np.zeros((args.height, args.width, 3), dtype=np.uint8)
        
        # 2. Update and render active particles
        active_particles = []
        for p in particles:
            p.age += 1
            
            # Physics: apply forces
            p.vx += wind_per_frame
            p.vy += gravity_per_frame
            
            # Drag coefficient to limit speed
            p.vx *= 0.98
            p.vy *= 0.98
            
            # Position update
            wobble = math.sin(p.age * p.wobble_speed) * p.wobble_amp
            p.x += p.vx + wobble
            p.y += p.vy
            
            # Rotation
            p.rot += p.rot_speed
            
            # Check boundaries and lifetime
            margin = int(max(sw, sh) * p.scale)
            if p.age >= p.lifetime:
                continue
            if p.x < -margin or p.x > args.width + margin:
                continue
            if args.gravity >= 0 and p.y > args.height + margin:
                continue
            if args.gravity < 0 and p.y < -margin:
                continue
                
            active_particles.append(p)
            
            # --- Rendering ---
            # Warp/Rotate and scale the sprite
            scale_sw = int(sw * p.scale)
            scale_sh = int(sh * p.scale)
            if scale_sw <= 0 or scale_sh <= 0:
                continue
                
            # Resize first
            resized_sprite = cv2.resize(sprite, (scale_sw, scale_sh), interpolation=cv2.INTER_LINEAR)
            
            # Rotate
            r_center = (scale_sw / 2.0, scale_sh / 2.0)
            rot_matrix = cv2.getRotationMatrix2D(r_center, p.rot, 1.0)
            warped = cv2.warpAffine(
                resized_sprite, 
                rot_matrix, 
                (scale_sw, scale_sh), 
                flags=cv2.INTER_LINEAR, 
                borderMode=cv2.BORDER_CONSTANT, 
                borderValue=(0, 0, 0, 0)
            )
            
            # Position calculations (centered on particle x, y)
            x0 = int(p.x - scale_sw / 2)
            y0 = int(p.y - scale_sh / 2)
            
            x0_f = max(0, x0)
            y0_f = max(0, y0)
            x1_f = min(args.width, x0 + scale_sw)
            y1_f = min(args.height, y0 + scale_sh)
            
            x0_s = x0_f - x0
            y0_s = y0_f - y0
            x1_s = x0_s + (x1_f - x0_f)
            y1_s = y0_s + (y1_f - y0_f)
            
            if (x1_f > x0_f) and (y1_f > y0_f):
                frame_patch = frame[y0_f:y1_f, x0_f:x1_f]
                sprite_patch = warped[y0_s:y1_s, x0_s:x1_s]
                
                s_rgb = sprite_patch[:, :, :3]
                s_alpha = sprite_patch[:, :, 3:4] / 255.0
                
                # Calculate fade in/out opacity
                opacity = 1.0
                age_sec = p.age / args.fps
                lifetime_sec = p.lifetime / args.fps
                if age_sec < args.fade_in:
                    opacity *= (age_sec / args.fade_in)
                if (lifetime_sec - age_sec) < args.fade_out:
                    opacity *= ((lifetime_sec - age_sec) / args.fade_out)
                    
                alpha = s_alpha * opacity
                
                # Blend onto black background
                blended = (s_rgb * alpha + frame_patch * (1.0 - alpha)).astype(np.uint8)
                frame[y0_f:y1_f, x0_f:x1_f] = blended
                
        particles = active_particles
        out.write(frame)
        
        # Simple console progress bar
        if (frame_idx + 1) % int(total_frames / 10 + 1) == 0 or frame_idx == total_frames - 1:
            progress = (frame_idx + 1) / total_frames * 100
            print(f"Progress: {progress:.0f}%")
            
    out.release()
    print(f"Success! Effect video saved to: {args.output}")

if __name__ == "__main__":
    main()
