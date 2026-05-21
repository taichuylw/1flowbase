import { SafetyCertificateOutlined } from '@ant-design/icons';
import { theme } from 'antd';
import React, { useMemo } from 'react';

// Seeded PRNG for SSR and client hydration consistency
function createRandom(seed: number) {
  let s = seed;
  return function() {
    let t = (s += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const PADS = [
  { id: 1, name: 'lower-left', cx: 85, cy: 115, rx: 32, ry: 14, branch: 'M 126,92 Q 100,105 85,115', thickness: 3 },
  { id: 2, name: 'lower-right', cx: 175, cy: 120, rx: 34, ry: 15, branch: 'M 130,92 Q 155,108 175,120', thickness: 3.2 },
  { id: 3, name: 'mid-left', cx: 70, cy: 80, rx: 28, ry: 12, branch: 'M 125,88 Q 95,85 70,80', thickness: 2.6 },
  { id: 4, name: 'mid-right', cx: 190, cy: 85, rx: 30, ry: 13, branch: 'M 131,88 Q 165,88 190,85', thickness: 2.8 },
  { id: 5, name: 'upper-left', cx: 105, cy: 55, rx: 26, ry: 11, branch: 'M 127,85 Q 115,70 105,55', thickness: 2.2 },
  { id: 6, name: 'upper-right', cx: 155, cy: 60, rx: 28, ry: 12, branch: 'M 129,85 Q 145,72 155,60', thickness: 2.4 },
  { id: 7, name: 'top-center', cx: 130, cy: 35, rx: 35, ry: 15, branch: 'M 128,85 Q 130,60 130,35', thickness: 2.5 }
];

const MINI_BRANCHES = [
  // Lower Left
  { d: 'M 85,115 Q 75,112 68,116', thickness: 1.5 },
  { d: 'M 85,115 Q 92,118 97,117', thickness: 1.2 },
  // Lower Right
  { d: 'M 175,120 Q 185,118 192,122', thickness: 1.5 },
  { d: 'M 175,120 Q 168,122 163,121', thickness: 1.2 },
  // Mid Left
  { d: 'M 70,80 Q 60,78 54,82', thickness: 1.2 },
  { d: 'M 70,80 Q 78,82 82,81', thickness: 1.0 },
  // Mid Right
  { d: 'M 190,85 Q 200,83 206,87', thickness: 1.3 },
  { d: 'M 190,85 Q 182,87 178,86', thickness: 1.0 },
  // Upper Left
  { d: 'M 105,55 Q 98,52 92,56', thickness: 1.0 },
  // Upper Right
  { d: 'M 155,60 Q 162,57 168,61', thickness: 1.0 },
  // Top Center
  { d: 'M 130,35 Q 120,32 114,35', thickness: 1.2 },
  { d: 'M 130,35 Q 140,32 146,35', thickness: 1.2 }
];

export function HeroAnimation() {
  const { token } = theme.useToken();

  // Generate deterministic leaf accents for each pad
  const padLeaves = useMemo(() => {
    const map: Record<number, Array<{ lx: number; ly: number; angle: number; size: number; color: string }>> = {};
    const rng = createRandom(54321);

    PADS.forEach(pad => {
      const list: Array<{ lx: number; ly: number; angle: number; size: number; color: string }> = [];
      const count = 5 + Math.floor(rng() * 4); // 5 to 8 leaves per pad
      for (let i = 0; i < count; i++) {
        const angle = (i / count) * Math.PI * 2 + (rng() - 0.5) * 0.4;
        const distOffset = 0.95 + rng() * 0.15; // slightly inside or outside the edge
        const lx = Math.cos(angle) * pad.rx * distOffset;
        const ly = Math.sin(angle) * pad.ry * distOffset;
        const size = 0.5 + rng() * 0.4;
        const leafRotation = angle * (180 / Math.PI) + 90 + (rng() - 0.5) * 45;

        const r = rng();
        let color = 'url(#canopy-light)';
        if (r < 0.25) {
          color = 'url(#canopy-mid)';
        } else if (r > 0.75) {
          color = 'url(#canopy-bright)';
        }

        list.push({ lx, ly, angle: leafRotation, size, color });
      }
      map[pad.id] = list;
    });
    return map;
  }, []);
  
  return (
    <div
      className="hero-animation-wrapper"
      style={{
        flex: 1,
        backgroundColor: '#f8fbf9', // Clean light background
        position: 'relative',
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      {/* Luminous bottom-left gradient glow */}
      <div
        style={{
          position: 'absolute',
          width: '70%',
          height: '70%',
          borderRadius: '50%',
          background: 'radial-gradient(circle, rgba(0, 200, 83, 0.07) 0%, rgba(43, 185, 177, 0.03) 50%, transparent 80%)',
          bottom: '-20%',
          left: '-20%',
          filter: 'blur(60px)',
          pointerEvents: 'none',
        }}
      />

      {/* Grid overlay for a subtle tech texture */}
      <div className="grid-overlay" />

      {/* Dynamic Wind-blown Drifting Leaves */}
      <div className="wind-leaves-container">
        {[...Array(20)].map((_, i) => {
          const duration = 8 + (i * 1.1) % 6;
          const delay = (i * 1.3) % 12;
          const top = 5 + (i * 17.7) % 80;
          const scale = 0.4 + (i * 0.15) % 0.8;
          const rotSpeed = 3 + (i * 0.9) % 6;
          
          return (
            <div
              key={i}
              className="wind-leaf"
              style={{
                top: `${top}%`,
                animationDelay: `${delay}s`,
                animationDuration: `${duration}s`,
              }}
            >
              <svg
                width="14"
                height="10"
                viewBox="0 0 14 10"
                style={{
                  transform: `scale(${scale})`,
                  animationDuration: `${rotSpeed}s`,
                  animationDelay: `${delay * 0.3}s`,
                }}
              >
                <path
                  d="M0,5 C4,0 10,0 14,5 C10,10 4,10 0,5 Z"
                  fill="url(#leaf-gradient)"
                />
              </svg>
            </div>
          );
        })}
      </div>

      {/* Floating subtle background shapes */}
      <div
        className="hero-shape shape-1"
        style={{
          position: 'absolute',
          width: '500px',
          height: '500px',
          borderRadius: '50%',
          background: `radial-gradient(circle at center, ${token.colorPrimary}10 0%, transparent 70%)`,
          top: '-10%',
          left: '10%',
          filter: 'blur(50px)',
        }}
      />
      <div
        className="hero-shape shape-2"
        style={{
          position: 'absolute',
          width: '600px',
          height: '600px',
          borderRadius: '50%',
          background: `radial-gradient(circle at center, ${token.colorInfo}06 0%, transparent 70%)`,
          top: '30%',
          right: '-10%',
          filter: 'blur(60px)',
        }}
      />

      {/* Center 3D Organic Lush Tree & Floating Island */}
      <div className="tree-container">
        {/* Glow behind the tree */}
        <div className="tree-glow" />

        {/* SVG Drawing for Tree, Island, and Ripple Rings */}
        <svg width="260" height="260" viewBox="0 0 260 260" style={{ position: 'absolute', top: 0, left: 0, zIndex: 5 }}>
          <defs>
            <radialGradient id="island-grad" cx="50%" cy="40%" r="50%">
              <stop offset="0%" stopColor="#7cb342" />
              <stop offset="70%" stopColor="#33691e" />
              <stop offset="100%" stopColor="#1b5e20" />
            </radialGradient>
            
            <linearGradient id="bark-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stopColor="#4a3b32" />
              <stop offset="50%" stopColor="#5c4a3f" />
              <stop offset="100%" stopColor="#332822" />
            </linearGradient>

            <radialGradient id="canopy-dark" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stopColor="#1e4620" />
              <stop offset="100%" stopColor="#0d240e" />
            </radialGradient>

            <radialGradient id="canopy-mid" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stopColor="#2e7d32" />
              <stop offset="100%" stopColor="#1b5e20" />
            </radialGradient>

            <radialGradient id="canopy-light" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stopColor="#81c784" />
              <stop offset="100%" stopColor="#2e7d32" />
            </radialGradient>

            <radialGradient id="canopy-bright" cx="40%" cy="40%" r="50%">
              <stop offset="0%" stopColor="#d4e157" />
              <stop offset="100%" stopColor="#66bb6a" />
            </radialGradient>

            <linearGradient id="leaf-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stopColor="#aed581" />
              <stop offset="100%" stopColor="#558b2f" />
            </linearGradient>
            
            <filter id="shadow" x="-10%" y="-10%" width="120%" height="120%">
              <feDropShadow dx="0" dy="4" stdDeviation="4" floodOpacity="0.12" floodColor="#0b240e" />
            </filter>

            <filter id="foliage-noise" x="-30%" y="-30%" width="160%" height="160%">
              <feTurbulence type="fractalNoise" baseFrequency="0.12" numOctaves="3" result="noise" />
              <feDisplacementMap in="SourceGraphic" in2="noise" scale="9" xChannelSelector="R" yChannelSelector="G" />
            </filter>
          </defs>

          {/* Luminous expanding ripple rings */}
          <ellipse cx="130" cy="220" rx="100" ry="24" className="ripple-ring ring-1" />
          <ellipse cx="130" cy="220" rx="100" ry="24" className="ripple-ring ring-2" />
          <ellipse cx="130" cy="220" rx="100" ry="24" className="ripple-ring ring-3" />

          {/* Floating mossy island earthy base */}
          <ellipse cx="130" cy="224" rx="86" ry="18" fill="#4d3a2e" opacity="0.9" />

          {/* Floating mossy island top */}
          <ellipse cx="130" cy="220" rx="88" ry="20" fill="url(#island-grad)" filter="url(#shadow)" />
          
          {/* Small grassy clumps on the island */}
          <ellipse cx="90" cy="216" rx="12" ry="4" fill="#558b2f" opacity="0.6" />
          <ellipse cx="170" cy="218" rx="15" ry="5" fill="#558b2f" opacity="0.6" />
          <ellipse cx="130" cy="222" rx="20" ry="6" fill="#689f38" opacity="0.5" />

          {/* Detailed grassy clumps */}
          <path d="M 85,218 Q 90,210 95,218" stroke="#7cb342" strokeWidth="1.5" fill="none" />
          <path d="M 88,218 Q 92,208 97,218" stroke="#81c784" strokeWidth="1.2" fill="none" />
          <path d="M 165,219 Q 170,212 175,219" stroke="#7cb342" strokeWidth="1.5" fill="none" />
          <path d="M 125,221 Q 130,214 135,221" stroke="#81c784" strokeWidth="1.5" fill="none" />
          <path d="M 132,222 Q 136,213 140,222" stroke="#7cb342" strokeWidth="1.2" fill="none" />

          {/* Tree Trunk & Branches (Static Gnarled Base) */}
          <g filter="url(#shadow)">
            {/* Roots */}
            <path d="M 116,220 C 110,220 100,221 94,225 C 104,221 114,215 118,205 Z" fill="url(#bark-gradient)" />
            <path d="M 144,220 C 150,220 160,221 166,225 C 156,221 146,215 142,205 Z" fill="url(#bark-gradient)" />
            
            {/* Main Trunk (static) */}
            <path d="M 120,220 C 122,190 124,165 120,140 C 116,115 124,100 128,88 C 132,100 137,112 138,130 C 140,150 137,185 140,220 Z" fill="url(#bark-gradient)" />
          </g>

          {/* Organic Lush Leafy Canopy & Branches (Swaying together) */}
          <g className="canopy-group" filter="url(#shadow)">
            {/* Main Branches */}
            {PADS.map(pad => (
              <path
                key={`branch-${pad.id}`}
                d={pad.branch}
                stroke="url(#bark-gradient)"
                strokeWidth={pad.thickness}
                strokeLinecap="round"
                fill="none"
              />
            ))}
            {MINI_BRANCHES.map((br, idx) => (
              <path
                key={`mini-branch-${idx}`}
                d={br.d}
                stroke="url(#bark-gradient)"
                strokeWidth={br.thickness}
                strokeLinecap="round"
                fill="none"
              />
            ))}

            {/* Foliage Pads */}
            {PADS.map(pad => {
              const { cx, cy, rx, ry } = pad;
              const leaves = padLeaves[pad.id] || [];
              return (
                <g key={`pad-group-${pad.id}`} className={`pad-group-${pad.id}`}>
                  {/* 1. Shadow layer (dark green, offset down) */}
                  <g filter="url(#foliage-noise)" opacity={0.35} transform="translate(0, 3)">
                    <ellipse cx={cx} cy={cy} rx={rx} ry={ry} />
                    <ellipse cx={cx - rx * 0.3} cy={cy + ry * 0.1} rx={rx * 0.7} ry={ry * 0.8} />
                    <ellipse cx={cx + rx * 0.3} cy={cy - ry * 0.1} rx={rx * 0.6} ry={ry * 0.8} />
                    <ellipse cx={cx} cy={cy - ry * 0.3} rx={rx * 0.8} ry={ry * 0.7} />
                  </g>

                  {/* 2. Back/Base layer */}
                  <g filter="url(#foliage-noise)" fill="url(#canopy-dark)" transform="translate(0, 1.5)">
                    <ellipse cx={cx} cy={cy} rx={rx} ry={ry} />
                    <ellipse cx={cx - rx * 0.3} cy={cy + ry * 0.1} rx={rx * 0.7} ry={ry * 0.8} />
                    <ellipse cx={cx + rx * 0.3} cy={cy - ry * 0.1} rx={rx * 0.6} ry={ry * 0.8} />
                    <ellipse cx={cx} cy={cy - ry * 0.3} rx={rx * 0.8} ry={ry * 0.7} />
                  </g>

                  {/* 3. Mid layer */}
                  <g filter="url(#foliage-noise)" fill="url(#canopy-mid)">
                    <ellipse cx={cx} cy={cy} rx={rx * 0.95} ry={ry * 0.95} />
                    <ellipse cx={cx - rx * 0.3} cy={cy + ry * 0.1} rx={rx * 0.65} ry={ry * 0.75} />
                    <ellipse cx={cx + rx * 0.3} cy={cy - ry * 0.1} rx={rx * 0.55} ry={ry * 0.75} />
                    <ellipse cx={cx} cy={cy - ry * 0.3} rx={rx * 0.75} ry={ry * 0.65} />
                  </g>

                  {/* 4. Front layer */}
                  <g filter="url(#foliage-noise)" fill="url(#canopy-light)" transform="translate(0, -1.5)">
                    <ellipse cx={cx} cy={cy - ry * 0.1} rx={rx * 0.85} ry={ry * 0.85} />
                    <ellipse cx={cx - rx * 0.25} cy={cy + ry * 0.05} rx={rx * 0.6} ry={ry * 0.7} />
                    <ellipse cx={cx + rx * 0.25} cy={cy - ry * 0.15} rx={rx * 0.5} ry={ry * 0.7} />
                    <ellipse cx={cx} cy={cy - ry * 0.35} rx={rx * 0.7} ry={ry * 0.6} />
                  </g>

                  {/* 5. Highlight layer */}
                  <g filter="url(#foliage-noise)" fill="url(#canopy-bright)" transform="translate(0, -3)">
                    <ellipse cx={cx} cy={cy - ry * 0.2} rx={rx * 0.7} ry={ry * 0.7} />
                    <ellipse cx={cx - rx * 0.2} cy={cy - ry * 0.05} rx={rx * 0.45} ry={ry * 0.55} />
                    <ellipse cx={cx + rx * 0.2} cy={cy - ry * 0.25} rx={rx * 0.4} ry={ry * 0.55} />
                    <ellipse cx={cx} cy={cy - ry * 0.4} rx={rx * 0.55} ry={ry * 0.5} />
                  </g>

                  {/* 6. Perimeter detail leaves */}
                  <g>
                    {leaves.map((l, idx) => (
                      <path
                        key={`p-leaf-${pad.id}-${idx}`}
                        d="M -3 0 C -3 -1.8, 3 -1.8, 3 0 C 3 1.8, -3 1.8, -3 0 Z"
                        transform={`translate(${cx + l.lx}, ${cy + l.ly}) rotate(${l.angle}) scale(${l.size})`}
                        fill={l.color}
                        opacity={0.92}
                      />
                    ))}
                  </g>
                </g>
              );
            })}
          </g>
        </svg>

        {/* Luminous floating data particles around the tree */}
        <div className="tree-particle tp-1" />
        <div className="tree-particle tp-2" style={{ background: 'radial-gradient(circle, rgba(43, 185, 177, 0.4) 0%, transparent 70%)' }} />
        <div className="tree-particle tp-3" />
        <div className="tree-particle tp-4" style={{ background: 'radial-gradient(circle, rgba(43, 185, 177, 0.4) 0%, transparent 70%)' }} />
      </div>

      {/* Logo & Tagline */}
      <div className="hero-text-block" style={{ zIndex: 10, textAlign: 'center', padding: '0 24px', width: '100%' }}>
        <div className="hero-tagline" aria-label="brand name and slogan">
          <div className="hero-text-line hero-title-line">1flowbase</div>
          <div className="hero-text-line hero-slogan-line">
            让每一次 <span style={{ color: '#00ab73', fontWeight: 600 }}>AI 聊天</span>，沉淀为可运行的<span style={{ color: '#00ab73', fontWeight: 600 }}>应用</span>。
          </div>
        </div>
      </div>

      {/* Bottom Security Badge */}
      <div className="security-badge">
        <SafetyCertificateOutlined style={{ color: '#00ab73', fontSize: 13 }} />
        <span>Your data is encrypted and secure.</span>
      </div>

      <style>
        {`
          .grid-overlay {
            position: absolute;
            top: 0; left: 0; right: 0; bottom: 0;
            background-image: radial-gradient(rgba(0, 171, 115, 0.03) 1px, transparent 1px);
            background-size: 24px 24px;
            opacity: 0.8;
            pointer-events: none;
          }

          /* Tree design */
          .tree-container {
            position: relative;
            width: 260px;
            height: 300px;
            margin-bottom: 24px;
            z-index: 5;
            animation: float-tree 8s ease-in-out infinite alternate;
          }

          .tree-glow {
            position: absolute;
            bottom: 30px;
            left: 50%;
            transform: translateX(-50%);
            width: 220px;
            height: 220px;
            border-radius: 50%;
            background: radial-gradient(circle, rgba(0, 200, 83, 0.16) 0%, rgba(43, 185, 177, 0.1) 60%, transparent 80%);
            filter: blur(30px);
            z-index: 1;
          }

          /* Ripple concentric rings expanding */
          .ripple-ring {
            stroke: rgba(0, 171, 115, 0.28);
            stroke-width: 1.2px;
            fill: none;
            transform-origin: 130px 220px;
            animation: ripple-out 5s linear infinite;
            filter: drop-shadow(0 0 2px rgba(0, 200, 83, 0.2));
          }
          .ring-1 { animation-delay: 0s; }
          .ring-2 { animation-delay: 1.66s; }
          .ring-3 { animation-delay: 3.33s; }

          @keyframes ripple-out {
            0% {
              transform: scale(0.6);
              opacity: 0;
            }
            15% {
              opacity: 1;
            }
            100% {
              transform: scale(1.4);
              opacity: 0;
            }
          }

          /* Swaying canopy logic */
          .canopy-group {
            transform-origin: 128px 90px;
            animation: sway-canopy 8s ease-in-out infinite alternate;
          }
          @keyframes sway-canopy {
            0% { transform: rotate(-1.5deg) skewX(-0.5deg); }
            100% { transform: rotate(2deg) skewX(1deg); }
          }

          /* Individual foliage pad secondary flutter animations */
          .pad-group-1 { animation: sway-p-1 7s ease-in-out infinite alternate; transform-origin: 85px 115px; }
          .pad-group-2 { animation: sway-p-2 8s ease-in-out infinite alternate; transform-origin: 175px 120px; }
          .pad-group-3 { animation: sway-p-3 6.5s ease-in-out infinite alternate; transform-origin: 70px 80px; }
          .pad-group-4 { animation: sway-p-4 7.5s ease-in-out infinite alternate; transform-origin: 190px 85px; }
          .pad-group-5 { animation: sway-p-5 8.5s ease-in-out infinite alternate; transform-origin: 105px 55px; }
          .pad-group-6 { animation: sway-p-6 9s ease-in-out infinite alternate; transform-origin: 155px 60px; }
          .pad-group-7 { animation: sway-p-7 8s ease-in-out infinite alternate; transform-origin: 130px 35px; }

          @keyframes sway-p-1 { 0% { transform: rotate(-1deg) translate(0, 0); } 100% { transform: rotate(1deg) translate(-1px, 1px); } }
          @keyframes sway-p-2 { 0% { transform: rotate(-0.8deg) translate(0, 0); } 100% { transform: rotate(1.2deg) translate(1px, 1px); } }
          @keyframes sway-p-3 { 0% { transform: rotate(-1.5deg) translate(0, 0); } 100% { transform: rotate(1.5deg) translate(-2px, -1px); } }
          @keyframes sway-p-4 { 0% { transform: rotate(-1.2deg) translate(0, 0); } 100% { transform: rotate(1.8deg) translate(2px, -1px); } }
          @keyframes sway-p-5 { 0% { transform: rotate(-2deg) translate(0, 0); } 100% { transform: rotate(1.5deg) translate(-1px, -2px); } }
          @keyframes sway-p-6 { 0% { transform: rotate(-1.5deg) translate(0, 0); } 100% { transform: rotate(2deg) translate(1px, -2px); } }
          @keyframes sway-p-7 { 0% { transform: rotate(-2.5deg) translate(0, 0); } 100% { transform: rotate(2deg) translate(0px, -3px); } }

          /* Wind leaves overlay and animations */
          .wind-leaves-container {
            position: absolute;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            pointer-events: none;
            overflow: hidden;
            z-index: 8;
          }

          .wind-leaf {
            position: absolute;
            left: -30px;
            animation-name: wind-drift;
            animation-iteration-count: infinite;
            animation-timing-function: linear;
            will-change: transform, opacity;
          }

          .wind-leaf svg {
            animation-name: leaf-spin;
            animation-iteration-count: infinite;
            animation-timing-function: ease-in-out;
            transform-origin: center;
            will-change: transform;
          }

          @keyframes wind-drift {
            0% {
              left: -30px;
              transform: translateY(0);
              opacity: 0;
            }
            10% {
              opacity: 0.9;
            }
            85% {
              opacity: 0.9;
            }
            100% {
              left: calc(100% + 30px);
              transform: translateY(-80px);
              opacity: 0;
            }
          }

          @keyframes leaf-spin {
            0% {
              transform: rotate(0deg) rotateX(0deg) translateY(0);
            }
            50% {
              transform: rotate(180deg) rotateX(60deg) translateY(12px);
            }
            100% {
              transform: rotate(360deg) rotateX(0deg) translateY(0);
            }
          }

          /* Particles */
          .tree-particle {
            position: absolute;
            border-radius: 50%;
            background: radial-gradient(circle, rgba(0, 200, 83, 0.4) 0%, transparent 70%);
            filter: blur(1.5px);
            pointer-events: none;
            z-index: 6;
          }
          .tp-1 { width: 12px; height: 12px; left: 15%; top: 35%; animation: float-particle 7s ease-in-out infinite; }
          .tp-2 { width: 14px; height: 14px; right: 15%; top: 25%; animation: float-particle 9s ease-in-out infinite 1.5s; }
          .tp-3 { width: 10px; height: 10px; left: 25%; bottom: 35%; animation: float-particle 6s ease-in-out infinite 3s; }
          .tp-4 { width: 13px; height: 13px; right: 22%; bottom: 45%; animation: float-particle 8s ease-in-out infinite 4.5s; }

          @keyframes float-tree {
            0% { transform: translateY(0px) rotate(0deg); }
            100% { transform: translateY(-10px) rotate(1.5deg); }
          }

          @keyframes shimmer-effect {
            0% { left: -100%; }
            30% { left: 150%; }
            100% { left: 150%; }
          }

          @keyframes float-particle {
            0%, 100% { transform: translateY(0) translateX(0); opacity: 0.4; }
            50% { transform: translateY(-20px) translateX(10px); opacity: 0.8; }
          }

          /* Text styling */
          .hero-text-block {
            animation: reveal 1.2s cubic-bezier(0.2, 0, 0, 1) forwards;
          }

          .hero-tagline {
            display: inline-flex;
            flex-direction: column;
            align-items: center;
            gap: 12px;
            line-height: 1.1;
          }

          .hero-text-line {
            font-weight: 800;
            margin: 0;
            letter-spacing: -0.01em;
          }

          .hero-title-line {
            font-size: 38px;
            color: #0f172a; /* Slate 900 */
            letter-spacing: -0.03em;
          }

          .hero-slogan-line {
            font-size: 18px;
            font-weight: 500;
            color: #64748b; /* Slate 500 */
          }

          .security-badge {
            position: absolute;
            bottom: 32px;
            left: 48px;
            display: flex;
            align-items: center;
            gap: 8px;
            color: #94a3b8; /* Slate 400 */
            font-size: 11px;
            pointer-events: none;
            animation: reveal 1.5s ease-out;
          }

          @keyframes reveal {
            0% { opacity: 0; transform: translateY(5px); }
            100% { opacity: 1; transform: translateY(0); }
          }

          .hero-shape {
            animation: float 22s ease-in-out infinite alternate;
          }
          .shape-1 {
            animation-delay: 0s;
            animation-duration: 25s;
          }
          .shape-2 {
            animation-delay: -5s;
            animation-duration: 30s;
            animation-direction: alternate-reverse;
          }

          @keyframes float {
            0% { transform: translate(0, 0) scale(1); }
            50% { transform: translate(25px, -35px) scale(1.05); }
            100% { transform: translate(0, 0) scale(1); }
          }
        `}
      </style>
    </div>
  );
}
