import "./App.css";

function App() {
	return (
		<div className="app">
			<main className="main">
				<div className="container">
					<header className="header">
						<h1 className="title">Ginseng</h1>
						<p className="subtitle">Free and direct file sharing, globally.</p>
					</header>

					<div className="content">
						<p className="description">
							Share files directly from your device—for free—with anyone, anywhere on the planet.
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
								Documentation
							</a>
						</div>
					</div>

					<footer className="footer">
						<p>
							Free and open source software •
							<a
								href="https://github.com/alDuncanson/ginseng"
								target="_blank"
								rel="noopener noreferrer"
							>
								View source
							</a>
						</p>
					</footer>
				</div>
			</main>
		</div>
	);
}

export default App;
