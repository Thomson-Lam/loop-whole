import { useEffect, useRef } from "react";
import Antigravity from "./Antigravity";

export default function App() {
  const progressRef = useRef(null);

  useEffect(() => {
    const bar = progressRef.current;
    const onScroll = () => {
      const h = document.documentElement;
      const max = h.scrollHeight - h.clientHeight;
      const p = max > 0 ? h.scrollTop / max : 0;
      if (bar) bar.style.transform = `scaleX(${p})`;
    };
    document.addEventListener("scroll", onScroll, { passive: true });
    onScroll();

    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add("in");
            io.unobserve(e.target);
          }
        });
      },
      { threshold: 0.15 }
    );
    document.querySelectorAll(".reveal").forEach((el) => io.observe(el));

    return () => {
      document.removeEventListener("scroll", onScroll);
      io.disconnect();
    };
  }, []);

  return (
    <>
      <div className="progress" ref={progressRef} />

      <header>
        <div className="wrap nav">
          <div className="brand">
            <span className="mark">✳</span> Loopey
          </div>
          <nav className="nav-links">
            <a href="#how">How it works</a>
            <a href="#">Docs</a>
            <a href="#">Dashboard</a>
          </nav>
          <a className="btn btn-primary" href="#">
            Launch app →
          </a>
        </div>
      </header>

      <main>
        <section className="hero" id="top">
          <div className="hero-bg">
            <Antigravity
              count={300}
              magnetRadius={8}
              ringRadius={18}
              waveSpeed={2.6}
              waveAmplitude={1}
              particleSize={1}
              lerpSpeed={0.05}
              color="#eaff00"
              autoAnimate
              particleVariance={1}
              rotationSpeed={1.7}
              depthFactor={1}
              pulseSpeed={3}
              particleShape="tetrahedron"
              fieldStrength={10}
            />
          </div>

          <div className="wrap hero-inner">
            <span className="spark">✳</span>
            <span className="mono hero-kicker">
              Context runtime · MCP-native · agent-agnostic
            </span>

            <h1>Loopey</h1>
            <p className="sub">
              A repository-state runtime that gives coding agents{" "}
              <b>only the context they need</b> — and surfaces evidence when they
              fail silently. No new prompts. No workflow changes.
            </p>

            <div className="cta-row">
              <a className="btn btn-primary" href="#">
                Launch →
              </a>
              <a className="btn btn-ghost" href="#how">
                How it works
              </a>
            </div>
          </div>

          <div className="wrap">
            <div className="stat-strip reveal">
              <div className="stat">
                <div className="num">
                  <span>42%</span>
                </div>
                <div className="lbl mono">Fewer tokens delivered</div>
              </div>
              <div className="stat">
                <div className="num">
                  0<span>×</span>
                </div>
                <div className="lbl mono">Lossy summarization</div>
              </div>
              <div className="stat">
                <div className="num">
                  1<span>·</span>drop-in
                </div>
                <div className="lbl mono">MCP gateway</div>
              </div>
            </div>
          </div>
        </section>

        <section className="how" id="how">
          <div className="wrap">
            <div className="section-head reveal">
              <span className="mono kicker">How it works</span>
              <h2>Right state, not more context.</h2>
              <p>
                Loopey sits between your agent and the repository as an MCP
                gateway. It remembers what the agent has already seen and
                delivers the smallest correct result.
              </p>
            </div>

            <div className="steps">
              <div className="step reveal">
                <span className="bar" />
                <div className="idx mono">01 — Intercept</div>
                <h3>Route every tool call</h3>
                <p>
                  Reads and writes flow through the Rust MCP gateway. Your agent
                  works exactly as before — nothing to configure.
                </p>
              </div>
              <div className="step reveal">
                <span className="bar" />
                <div className="idx mono">02 — Compact</div>
                <h3>Send only what changed</h3>
                <p>
                  Unchanged files are suppressed, edits are delivered as diffs,
                  and unseen files return in full. Deterministic, never lossy.
                </p>
              </div>
              <div className="step reveal">
                <span className="bar" />
                <div className="idx mono">03 — Reveal</div>
                <h3>See savings &amp; evidence</h3>
                <p>
                  A live dashboard shows tokens saved per call and flags evidence
                  of silent failures — stale state, retry loops, unresolved
                  tests.
                </p>
              </div>
            </div>
          </div>
        </section>
      </main>

      <footer>
        <div className="wrap foot">
          <div className="brand">
            <span className="mark">✳</span> Loopey
          </div>
          <span className="mono">Built at Hack the 6ix · 2026</span>
        </div>
      </footer>
    </>
  );
}
