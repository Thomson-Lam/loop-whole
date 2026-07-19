import { useEffect, useRef } from "react";
import Antigravity from "./Antigravity";
import ContextTimeline from "./ContextTimeline";
import ToolReplay from "./ToolReplay";

export default function Landing() {
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
          <nav className="nav-links">
            <a href="#how">How it works</a>
            <a href="#replay">Example</a>
            <a href="#/benchmarks">Benchmarks</a>
          </nav>
          <img className="event-logo" src="/ht6.svg" alt="Hack the 6ix" />
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
            <h1>Loop-Whole</h1>
            <p className="sub">
              An agent-agnostic repository runtime that <b>cuts AI compute costs</b>{" "}
              and surfaces <b>silent execution failures</b> — without new prompts
              or lossy summarization.
            </p>

            <div className="cta-row">
              <a className="btn btn-primary" href="#/app">
                Launch →
              </a>
              <a className="btn btn-ghost" href="#/benchmarks">
                View benchmarks
              </a>
              <a className="btn btn-ghost" href="#how">
                How it works
              </a>
            </div>
          </div>

          <div className="wrap">
            <div className="stat-strip reveal">
              <div className="stat">
                <div className="num">Reduce token costs</div>
                <div className="lbl mono">With no performance degradation</div>
              </div>
              <div className="stat">
                <div className="num">0 LLM calls</div>
                <div className="lbl mono">Relative compaction</div>
              </div>
              <div className="stat">
                <div className="num">Plug and play</div>
                <div className="lbl mono">Experience</div>
              </div>
            </div>
          </div>
        </section>

        <section className="how" id="how">
          <div className="wrap">
            <div className="section-head reveal">
              <h2>How it works</h2>
            </div>

            <ContextTimeline />
          </div>
        </section>

        <ToolReplay />
      </main>

      <footer>
        <div className="wrap foot">
          <span className="mono">Made with ❤️ by Henrique, Thomson, Jonathan @ HT6 26</span>
        </div>
      </footer>
    </>
  );
}
