import { theme } from 'antd';
import React from 'react';

export function HeroAnimation() {
  const { token } = theme.useToken();

  return (
    <div
      style={{
        flex: 1,
        backgroundColor: token.colorBgLayout,
        position: 'relative',
        overflow: 'hidden',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <div
        className="hero-shape shape-1"
        style={{
          position: 'absolute',
          width: '600px',
          height: '600px',
          borderRadius: '50%',
          background: `radial-gradient(circle at center, ${token.colorPrimary}40 0%, transparent 70%)`,
          top: '-10%',
          left: '-10%',
          filter: 'blur(40px)',
        }}
      />
      <div
        className="hero-shape shape-2"
        style={{
          position: 'absolute',
          width: '800px',
          height: '800px',
          borderRadius: '50%',
          background: `radial-gradient(circle at center, ${token.colorInfo}30 0%, transparent 70%)`,
          bottom: '-20%',
          right: '-10%',
          filter: 'blur(60px)',
        }}
      />
      <div
        className="hero-shape shape-3"
        style={{
          position: 'absolute',
          width: '400px',
          height: '400px',
          borderRadius: '50%',
          background: `radial-gradient(circle at center, ${token.colorSuccess}20 0%, transparent 70%)`,
          top: '40%',
          left: '60%',
          filter: 'blur(30px)',
        }}
      />

      <div className="hero-text-block" style={{ zIndex: 1, textAlign: 'center', padding: `0 clamp(12px, 6vw, 80px)`, width: '100%', marginTop: '-15vh' }}>
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
            --hero-slogan-size: clamp(1rem, 2.8vw, 3rem);
            --hero-title-size: calc(var(--hero-slogan-size) * 7 / 3);
            display: inline-flex;
            flex-direction: column;
            align-items: center;
            gap: clamp(0.2rem, 0.8vw, 1.2rem);
            line-height: 1.2;
            font-family: ${token.fontFamily};
          }

          .hero-text-line {
            background: linear-gradient(
              115deg,
              var(--hero-color-blue) 0%,
              var(--hero-color-blue) 25%,
              var(--hero-color-glow) 30%,
              var(--hero-color-green) 35%,
              var(--hero-color-green) 65%,
              var(--hero-color-glow) 70%,
              var(--hero-color-blue) 75%,
              var(--hero-color-blue) 100%
            );
            background-size: 500% 100%;
            background-repeat: no-repeat;
            color: transparent;
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
            animation: shine 12s linear infinite;
            margin: 0;
            line-height: 1.15;
            --hero-color-blue: rgba(59, 130, 246, 0.8);
            --hero-color-glow: #e6f7f2;
            --hero-color-green: ${token.colorPrimary};
          }

          .hero-title-line {
            font-size: var(--hero-title-size);
            font-weight: 800;
            letter-spacing: -0.03em;
          }

          .hero-slogan-line {
            font-size: var(--hero-slogan-size);
            font-weight: 500;
            letter-spacing: 0.08em;
          }

          @keyframes fadeIn {
            0% { opacity: 0; transform: translateY(15px); }
            100% { opacity: 1; transform: translateY(0); }
          }

          @keyframes shine {
            0%, 5% { background-position: 100% center; }
            50%, 55% { background-position: 50% center; }
            100% { background-position: 0% center; }
          }

          .hero-shape {
            animation: float 20s ease-in-out infinite alternate;
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
          .shape-3 {
            animation-delay: -10s;
            animation-duration: 20s;
          }

          @keyframes float {
            0% {
              transform: translate(0, 0) scale(1);
            }
            33% {
              transform: translate(30px, -50px) scale(1.1);
            }
            66% {
              transform: translate(-20px, 20px) scale(0.9);
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
