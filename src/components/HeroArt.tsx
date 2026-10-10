const rand = (i: number, n: number) => {
  const x = Math.sin(i * 127.1 + n * 311.7) * 43758.5453;
  return x - Math.floor(x);
};

const makeDiamonds = (count: number) => Array.from({ length: count }, (_, i) => {
  const w = 26 + rand(i, 1) * 52;
  const half = w * 0.95;
  return {
    left: 8 + rand(i, 2) * 84,
    top: 14 + rand(i, 3) * 66,
    w,
    slant: Math.sqrt(half * half + (w / 2) ** 2),
    tilt: (Math.atan2(w / 2, half) * 180) / Math.PI,
    spin: 11 + rand(i, 4) * 14,
    float: 6 + rand(i, 5) * 6,
    delay: -rand(i, 6) * 20,
    reverse: i % 2 === 1,
    alpha: 0.55 + rand(i, 7) * 0.45,
  };
});

const DIAMONDS = makeDiamonds(7);
const BOOT_DIAMONDS = makeDiamonds(18);

const ORBS = Array.from({ length: 5 }, (_, i) => ({
  left: rand(i, 8) * 100,
  top: rand(i, 9) * 100,
  size: 120 + rand(i, 10) * 160,
  drift: 14 + rand(i, 11) * 14,
  delay: -rand(i, 12) * 20,
}));

const FACE_SHADES = [0.6, 0.22, 0.4, 0.14];

export default function HeroArt({ colors, dense }: { colors: [string, string]; dense?: boolean }) {
  const list = dense ? BOOT_DIAMONDS : DIAMONDS;
  return (
    <div className="hero-art" aria-hidden="true" style={{ background: `linear-gradient(135deg, ${colors[0]}, ${colors[1]})` }}>
      {ORBS.map((o, i) => (
        <i key={i} className="hero-orb" style={{ left: `${o.left}%`, top: `${o.top}%`, width: o.size, height: o.size, animationDuration: `${o.drift}s`, animationDelay: `${o.delay}s` }} />
      ))}
      {list.map((d, i) => (
        <span
          key={i}
          className="dia-wrap"
          style={{
            left: `${d.left}%`,
            top: `${d.top}%`,
            opacity: d.alpha,
            animationDuration: `${d.float}s`,
            animationDelay: `${d.delay}s`,
            ["--w" as string]: `${d.w}px`,
            ["--l" as string]: `${d.slant}px`,
            ["--t" as string]: `${d.tilt}deg`,
          }}
        >
          <span className="dia" style={{ animationDuration: `${d.spin}s`, animationDelay: `${d.delay}s`, animationDirection: d.reverse ? "reverse" : "normal" }}>
            {[0, 1, 2, 3].map((n) => (
              <i key={`u${n}`} className="face up" style={{ ["--r" as string]: `${n * 90}deg`, ["--s" as string]: FACE_SHADES[n] }} />
            ))}
            {[0, 1, 2, 3].map((n) => (
              <i key={`d${n}`} className="face down" style={{ ["--r" as string]: `${n * 90}deg`, ["--s" as string]: FACE_SHADES[(n + 1) % 4] * 0.8 }} />
            ))}
          </span>
        </span>
      ))}
    </div>
  );
}
