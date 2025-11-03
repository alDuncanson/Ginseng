import "./App.css";

function App() {
	return (
		<div className="app">
			<main className="main">
				<div className="split-container">
					<div className="content-side">
						<h1 className="title">Ginseng</h1>
						<p className="subtitle">Free and direct file sharing, globally</p>
						<p className="description">
							Share files directly from your device—for free—with anyone, anywhere on the
							planet.
						</p>
						<div className="actions">
							<a
								href="https://github.com/alDuncanson/ginseng/releases/latest"
								className="button button-primary"
								target="_blank"
								rel="noopener noreferrer"
							>
								Download
							</a>
							<a
								href="https://github.com/alDuncanson/ginseng"
								className="button button-secondary"
								target="_blank"
								rel="noopener noreferrer"
							>
								Docs
							</a>
						</div>
					</div>
					<div className="demo-side">
						<img src="/Ginseng/demo.gif" alt="Ginseng demo" className="demo-gif" />
					</div>
				</div>
				<footer className="footer">
					<p>
						Free and open source software •{" "}
						<a
							href="https://github.com/alDuncanson/ginseng"
							target="_blank"
							rel="noopener noreferrer"
						>
							View source
						</a>
					</p>
				</footer>
			</main>
		</div>
	);
}

export default App;
