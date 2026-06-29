import Link from 'next/link';

export default function HomePage() {
  return (
    <main className="home">
      <div className="home__inner">
        <nav className="home__nav" aria-label="Main">
          <Link className="home__brand" href="/">
            KNXyz
          </Link>
          <Link className="home__link" href="/docs">
            Docs
          </Link>
        </nav>

        <section className="home__hero">
          <h1>KNX libraries for Rust, Python, and Node.js.</h1>
          <p>
            KNXyz provides KNX datapoint codecs and KNXnet/IP client building
            blocks for gateway discovery, tunnel connections, group reads, and
            group writes.
          </p>
          <div className="home__actions">
            <Link className="home__button home__button--primary" href="/docs">
              Read the docs
            </Link>
            <Link className="home__button" href="/docs/install">
              Install
            </Link>
          </div>
        </section>

        <section className="home__grid" aria-label="Highlights">
          <div>
            <h2>Datapoint codecs</h2>
            <p>Encode values into KNX payload bytes and decode them back.</p>
          </div>
          <div>
            <h2>KNXnet/IP</h2>
            <p>Discover gateways, connect through tunnels, and operate groups.</p>
          </div>
          <div>
            <h2>Package surfaces</h2>
            <p>Use KNXyz from Rust, Python, Node.js, TypeScript, C, and Cython.</p>
          </div>
        </section>
      </div>
    </main>
  );
}
