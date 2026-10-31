// Next.js instrumentation — يُستدعى مرة واحدة عند إقلاع خادم dev/الإنتاج.
// يشغّل طيار أوين الآلي فوراً حتى لو لم يفتح أحد الموقع بعد.
export async function register(): Promise<void> {
  if (process.env.NEXT_RUNTIME !== 'nodejs') return;
  try {
    const { ensureAutopilot } = await import('@/lib/autopilot');
    ensureAutopilot();
    console.log('[owen] autopilot armed via instrumentation');
  } catch (e) {
    console.error('[owen] autopilot boot failed:', e);
  }
}
