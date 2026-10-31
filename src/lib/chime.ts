const NOTES = [523.25, 659.25, 783.99, 1046.5, 1318.51];

export function playChime(): () => void {
  let ctx: AudioContext;
  try {
    ctx = new AudioContext();
  } catch {
    return () => undefined;
  }
  void ctx.resume().catch(() => undefined);

  const master = ctx.createGain();
  master.gain.value = 0.16;
  master.connect(ctx.destination);

  const now = ctx.currentTime + 0.05;

  const swell = ctx.createOscillator();
  const swellGain = ctx.createGain();
  swell.type = "sine";
  swell.frequency.setValueAtTime(130, now);
  swell.frequency.exponentialRampToValueAtTime(520, now + 1.1);
  swellGain.gain.setValueAtTime(0.0001, now);
  swellGain.gain.exponentialRampToValueAtTime(0.5, now + 0.9);
  swellGain.gain.exponentialRampToValueAtTime(0.0001, now + 1.5);
  swell.connect(swellGain).connect(master);
  swell.start(now);
  swell.stop(now + 1.6);

  NOTES.forEach((freq, i) => {
    const t = now + 0.55 + i * 0.11;
    for (const [mult, level, type] of [[1, 0.6, "sine"], [2, 0.18, "sine"], [3.01, 0.08, "triangle"]] as const) {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = type;
      osc.frequency.value = freq * mult;
      gain.gain.setValueAtTime(0.0001, t);
      gain.gain.exponentialRampToValueAtTime(level, t + 0.015);
      gain.gain.exponentialRampToValueAtTime(0.0001, t + 1.1);
      osc.connect(gain).connect(master);
      osc.start(t);
      osc.stop(t + 1.2);
    }
  });

  return () => {
    master.gain.cancelScheduledValues(ctx.currentTime);
    master.gain.setTargetAtTime(0, ctx.currentTime, 0.05);
    setTimeout(() => void ctx.close().catch(() => undefined), 300);
  };
}

export function playAlarm(level: 1 | 2): () => void {
  let ctx: AudioContext;
  try {
    ctx = new AudioContext();
  } catch {
    return () => undefined;
  }
  void ctx.resume().catch(() => undefined);
  const master = ctx.createGain();
  master.gain.value = level === 1 ? 0.12 : 0.2;
  master.connect(ctx.destination);
  const now = ctx.currentTime + 0.03;

  if (level === 1) {
    const osc = ctx.createOscillator();
    osc.type = "square";
    for (let i = 0; i < 6; i += 1) osc.frequency.setValueAtTime(i % 2 === 0 ? 880 : 620, now + i * 0.3);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.0001, now);
    gain.gain.exponentialRampToValueAtTime(1, now + 0.03);
    gain.gain.setValueAtTime(1, now + 1.6);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + 1.85);
    osc.connect(gain).connect(master);
    osc.start(now);
    osc.stop(now + 1.9);
  } else {
    const siren = ctx.createOscillator();
    siren.type = "sawtooth";
    for (let i = 0; i < 5; i += 1) {
      siren.frequency.setValueAtTime(420, now + i * 0.55);
      siren.frequency.linearRampToValueAtTime(1500, now + i * 0.55 + 0.5);
    }
    const sirenGain = ctx.createGain();
    sirenGain.gain.setValueAtTime(0.0001, now);
    sirenGain.gain.exponentialRampToValueAtTime(1, now + 0.04);
    sirenGain.gain.setValueAtTime(1, now + 2.5);
    sirenGain.gain.exponentialRampToValueAtTime(0.0001, now + 2.9);
    siren.connect(sirenGain).connect(master);
    siren.start(now);
    siren.stop(now + 3);

    const pulse = ctx.createOscillator();
    pulse.type = "square";
    pulse.frequency.value = 110;
    const pulseGain = ctx.createGain();
    for (let i = 0; i < 10; i += 1) {
      pulseGain.gain.setValueAtTime(0.5, now + i * 0.28);
      pulseGain.gain.setValueAtTime(0, now + i * 0.28 + 0.14);
    }
    pulse.connect(pulseGain).connect(master);
    pulse.start(now);
    pulse.stop(now + 2.9);
  }

  return () => {
    master.gain.cancelScheduledValues(ctx.currentTime);
    master.gain.setTargetAtTime(0, ctx.currentTime, 0.04);
    setTimeout(() => void ctx.close().catch(() => undefined), 250);
  };
}

export function playBell(): () => void {
  let ctx: AudioContext;
  try {
    ctx = new AudioContext();
  } catch {
    return () => undefined;
  }
  void ctx.resume().catch(() => undefined);
  const master = ctx.createGain();
  master.gain.value = 0.14;
  master.connect(ctx.destination);
  const now = ctx.currentTime + 0.03;
  [[0, 1568], [0.16, 2093]].forEach(([delay, freq]) => {
    for (const [mult, level, decay] of [[1, 0.6, 1.4], [2.76, 0.22, 0.9], [5.4, 0.1, 0.5]] as const) {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = freq * mult;
      const start = now + delay;
      gain.gain.setValueAtTime(0.0001, start);
      gain.gain.exponentialRampToValueAtTime(level, start + 0.008);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + decay);
      osc.connect(gain).connect(master);
      osc.start(start);
      osc.stop(start + decay + 0.05);
    }
  });
  return () => {
    master.gain.cancelScheduledValues(ctx.currentTime);
    master.gain.setTargetAtTime(0, ctx.currentTime, 0.04);
    setTimeout(() => void ctx.close().catch(() => undefined), 250);
  };
}
