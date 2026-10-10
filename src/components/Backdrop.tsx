const rand = (i: number, n: number) => {
  const x = Math.sin(i * 97.13 + n * 31.7) * 10000;
  return x - Math.floor(x);
};

const ITEMS = Array.from({ length: 18 }, (_, i) => ({
  left: rand(i, 1) * 100,
  top: rand(i, 2) * 100,
  size: 28 + rand(i, 3) * 96,
  duration: 22 + rand(i, 4) * 26,
  delay: -rand(i, 5) * 40,
  rotate: rand(i, 6) * 70 - 35,
  opacity: 0.22 + rand(i, 7) * 0.4,
  orb: i % 3 === 0,
}));

export default function Backdrop() {
  return (
    <div className="backdrop" aria-hidden="true">
      {ITEMS.map((s, i) => (
        <i
          key={i}
          className={s.orb ? "orb" : "shard"}
          style={{
            left: `${s.left}%`,
            top: `${s.top}%`,
            width: s.size,
            height: s.orb ? s.size : s.size * 1.45,
            opacity: s.opacity,
            animationDuration: `${s.duration}s`,
            animationDelay: `${s.delay}s`,
            ["--rot" as string]: `${s.rotate}deg`,
          }}
        />
      ))}
    </div>
  );
}
