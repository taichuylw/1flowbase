import { theme } from 'antd';
import React, { useEffect, useRef } from 'react';

interface Leaf {
  x: number;
  y: number;
  vx: number;
  vy: number;
  baseVx: number;
  baseVy: number;
  rotation: number;
  rotationSpeed: number;
  scaleX: number;
  scaleXSpeed: number;
  size: number;
  color: string;
  veinColor: string;
  flutterPhase: number;
  flutterSpeed: number;
}

export function HeroAnimation() {
  const { token } = theme.useToken();
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const mouseRef = useRef({ x: -1000, y: -1000, vx: 0, vy: 0 });
  const lastMousePos = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let animationFrameId: number;
    let leaves: Leaf[] = [];
    const leafCount = 55; // Increased to 55 leaves

    // Resize handler to adjust canvas bounds with retina display support
    const resizeCanvas = () => {
      const rect = container.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      canvas.width = rect.width * dpr;
      canvas.height = rect.height * dpr;
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = `${rect.height}px`;
      ctx.scale(dpr, dpr);
    };

    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);

    // Glowing fluorescent green theme colors matching the login button (#00d084)
    const colors = [
      `rgba(0, 208, 132, 0.28)`, // Vibrant Fluorescent Green
      `rgba(0, 208, 132, 0.20)`, // Medium Fluorescent Green
      `rgba(52, 211, 153, 0.24)`, // Emerald Green
      `rgba(209, 250, 229, 0.35)`, // Pale Sage / White-green
    ];
    const veinColors = [
      `rgba(0, 208, 132, 0.45)`,
      `rgba(0, 208, 132, 0.35)`,
      `rgba(52, 211, 153, 0.40)`,
      `rgba(16, 185, 129, 0.38)`,
    ];

    const createLeaf = (initYRandom = false): Leaf => {
      const rect = container.getBoundingClientRect();
      const width = rect.width || 800;
      const height = rect.height || 600;
      
      const idx = Math.floor(Math.random() * colors.length);
      
      return {
        x: Math.random() * width,
        y: initYRandom ? Math.random() * height : -20 - Math.random() * 50,
        vx: Math.random() * 0.4 - 0.2,
        vy: Math.random() * 0.8 + 0.4,
        baseVx: Math.random() * 0.2 + 0.05, // Slight drift to the right
        baseVy: Math.random() * 0.55 + 0.35, // Falling speed
        rotation: Math.random() * Math.PI * 2,
        rotationSpeed: (Math.random() * 0.02 - 0.01) * 0.4,
        scaleX: Math.random() * 2 - 1,
        scaleXSpeed: Math.random() * 0.01 + 0.005,
        size: Math.random() * 7 + 5, // 5px to 12px size
        color: colors[idx],
        veinColor: veinColors[idx],
        flutterPhase: Math.random() * Math.PI * 2,
        flutterSpeed: Math.random() * 0.012 + 0.004,
      };
    };

    // Populate initially scattered leaves
    for (let i = 0; i < leafCount; i++) {
      leaves.push(createLeaf(true));
    }

    let time = 0;

    const animate = () => {
      time += 1;
      
      const rect = container.getBoundingClientRect();
      const width = rect.width;
      const height = rect.height;

      ctx.clearRect(0, 0, width, height);

      // Decay mouse velocity
      mouseRef.current.vx *= 0.94;
      mouseRef.current.vy *= 0.94;

      // Update and draw leaves
      leaves.forEach((leaf) => {
        // Base forces: gravity + light wind drift
        leaf.vy += (leaf.baseVy - leaf.vy) * 0.04;
        leaf.vx += (leaf.baseVx - leaf.vx) * 0.04;

        // Sway (fluttering) using sine wave
        const sway = Math.sin(time * leaf.flutterSpeed + leaf.flutterPhase) * 0.25;
        leaf.vx += sway * 0.15;
        leaf.rotation += leaf.rotationSpeed + sway * 0.003;
        leaf.scaleX = Math.sin(time * leaf.scaleXSpeed + leaf.flutterPhase);

        // Mouse interaction
        const dx = leaf.x - mouseRef.current.x;
        const dy = leaf.y - mouseRef.current.y;
        const distSq = dx * dx + dy * dy;
        const radius = 150; // Radius of mouse influence

        if (distSq < radius * radius) {
          const dist = Math.sqrt(distSq);
          const force = 1 - dist / radius; // 0 to 1

          // 1. Mouse motion drag (wind)
          leaf.vx += mouseRef.current.vx * force * 0.6;
          leaf.vy += mouseRef.current.vy * force * 0.6;

          // 2. Physical push away (repulsion)
          const dirX = dx / (dist || 1);
          const dirY = dy / (dist || 1);
          leaf.vx += dirX * force * 0.4;
          leaf.vy += dirY * force * 0.18;
        }

        // Apply friction/air resistance
        leaf.vx *= 0.97;
        leaf.vy *= 0.97;

        // Limit maximum speeds
        const speedSq = leaf.vx * leaf.vx + leaf.vy * leaf.vy;
        const maxSpeed = 6.5;
        if (speedSq > maxSpeed * maxSpeed) {
          const speed = Math.sqrt(speedSq);
          leaf.vx = (leaf.vx / speed) * maxSpeed;
          leaf.vy = (leaf.vy / speed) * maxSpeed;
        }

        // Update coordinates
        leaf.x += leaf.vx;
        leaf.y += leaf.vy;

        // Boundary checks
        if (leaf.y > height + 20) {
          Object.assign(leaf, createLeaf(false));
        }
        if (leaf.x < -20) {
          leaf.x = width + 10;
        } else if (leaf.x > width + 20) {
          leaf.x = -10;
        }

        // Draw leaf with soft fluorescent shadow glow
        ctx.save();
        ctx.translate(leaf.x, leaf.y);
        ctx.rotate(leaf.rotation);
        ctx.scale(leaf.scaleX, 1);

        // Enable glowing shadow only for the leaf drawing
        ctx.shadowColor = 'rgba(0, 208, 132, 0.4)';
        ctx.shadowBlur = 5;

        // Draw leaf silhouette
        ctx.beginPath();
        ctx.moveTo(0, -leaf.size);
        ctx.quadraticCurveTo(-leaf.size * 0.7, -leaf.size * 0.2, 0, leaf.size);
        ctx.quadraticCurveTo(leaf.size * 0.7, -leaf.size * 0.2, 0, -leaf.size);
        ctx.fillStyle = leaf.color;
        ctx.fill();

        // Draw center vein (disable shadow for vein line to keep it crisp)
        ctx.shadowBlur = 0;
        ctx.beginPath();
        ctx.moveTo(0, -leaf.size);
        ctx.lineTo(0, leaf.size * 0.8);
        ctx.strokeStyle = leaf.veinColor;
        ctx.lineWidth = 0.8;
        ctx.stroke();

        ctx.restore();
      });

      animationFrameId = requestAnimationFrame(animate);
    };

    animate();

    return () => {
      window.removeEventListener('resize', resizeCanvas);
      cancelAnimationFrame(animationFrameId);
    };
  }, []);

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    const container = containerRef.current;
    if (!container) return;

    const rect = container.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (lastMousePos.current) {
      const vx = x - lastMousePos.current.x;
      const vy = y - lastMousePos.current.y;
      
      mouseRef.current.vx = vx;
      mouseRef.current.vy = vy;
    }

    mouseRef.current.x = x;
    mouseRef.current.y = y;
    lastMousePos.current = { x, y };
  };

  const handleMouseLeave = () => {
    mouseRef.current.x = -1000;
    mouseRef.current.y = -1000;
    mouseRef.current.vx = 0;
    mouseRef.current.vy = 0;
    lastMousePos.current = null;
  };

  return (
    <div
      ref={containerRef}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
      style={{
        flex: 1,
        background: 'linear-gradient(135deg, #ffffff 40%, #e6f7f2 100%)', // Brighter white to mint-green gradient
        position: 'relative',
        overflow: 'hidden',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        backgroundImage: `radial-gradient(rgba(0, 208, 132, 0.05) 1px, transparent 1px)`, // Light emerald dots for grid texture
        backgroundSize: '24px 24px',
      }}
    >
      <canvas
        ref={canvasRef}
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          width: '100%',
          height: '100%',
          pointerEvents: 'none',
          zIndex: 0,
        }}
      />

      <div
        className="hero-shape shape-1"
        style={{
          position: 'absolute',
          width: '600px',
          height: '600px',
          borderRadius: '50%',
          background: 'radial-gradient(circle at center, rgba(0, 208, 132, 0.14) 0%, transparent 70%)',
          top: '-10%',
          left: '-10%',
          filter: 'blur(60px)',
          pointerEvents: 'none',
        }}
      />
      <div
        className="hero-shape shape-2"
        style={{
          position: 'absolute',
          width: '800px',
          height: '800px',
          borderRadius: '50%',
          background: 'radial-gradient(circle at center, rgba(0, 162, 255, 0.12) 0%, transparent 70%)',
          bottom: '-20%',
          right: '-10%',
          filter: 'blur(80px)',
          pointerEvents: 'none',
        }}
      />
      <div
        className="hero-shape shape-3"
        style={{
          position: 'absolute',
          width: '400px',
          height: '400px',
          borderRadius: '50%',
          background: 'radial-gradient(circle at center, rgba(52, 211, 153, 0.07) 0%, transparent 70%)',
          top: '40%',
          left: '60%',
          filter: 'blur(50px)',
          pointerEvents: 'none',
        }}
      />

      <div className="hero-text-block" style={{ zIndex: 1, textAlign: 'center', padding: `0 clamp(12px, 6vw, 80px)`, width: '100%', marginTop: '-15vh', pointerEvents: 'none' }}>
        <div className="hero-tagline" aria-label="brand name and slogan">
          <div className="hero-text-line hero-title-line">1flowbase</div>
          <div className="hero-text-line hero-slogan-line">对话即是壁垒，AI应用原生底座</div>
        </div>
      </div>

      <style>
        {`
          .hero-text-block {
            animation: fadeIn 1.5s cubic-bezier(0.16, 1, 0.3, 1) forwards;
          }

          .hero-tagline {
            --hero-slogan-size: clamp(1.2rem, 2.2vw, 2.5rem);
            --hero-title-size: calc(var(--hero-slogan-size) * 2.2);
            display: inline-flex;
            flex-direction: column;
            align-items: center;
            gap: clamp(0.5rem, 1vw, 1.5rem);
            line-height: 1.2;
            font-family: ${token.fontFamily};
          }

          .hero-title-line {
            font-size: var(--hero-title-size);
            font-weight: 800;
            letter-spacing: -0.02em;
            background: linear-gradient(135deg, #00d084 10%, ${token.colorText} 80%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            margin: 0;
            line-height: 1.15;
          }

          .hero-slogan-line {
            font-size: var(--hero-slogan-size);
            font-weight: 400;
            letter-spacing: 0.06em;
            color: ${token.colorTextSecondary};
            margin: 0;
            line-height: 1.15;
          }

          @keyframes fadeIn {
            0% { opacity: 0; transform: translateY(15px); }
            100% { opacity: 1; transform: translateY(0); }
          }

          .hero-shape {
            animation: float 25s ease-in-out infinite alternate;
          }
          .shape-1 {
            animation-delay: 0s;
            animation-duration: 25s;
          }
          .shape-2 {
            animation-delay: -5s;
            animation-duration: 35s;
            animation-direction: alternate-reverse;
          }
          .shape-3 {
            animation-delay: -10s;
            animation-duration: 20s;
          }

          @keyframes float {
            0% {
              transform: translate(0, 0) scale(1);
            }
            33% {
              transform: translate(25px, -40px) scale(1.05);
            }
            66% {
              transform: translate(-15px, 15px) scale(0.95);
            }
            100% {
              transform: translate(0, 0) scale(1);
            }
          }
        `}
      </style>
    </div>
  );
}
