import "./App.css";

function App() {
	return (
		<div className="app">
			<main className="main">
				<div className="container">
					<header className="header">
						<h1 className="title">Ginseng</h1>
						<p className="subtitle">
							Peer-to-peer file sharing with cryptographic sovereignty
						</p>
					</header>

					<div className="content">
						<p className="description">
							Eliminate intermediaries and restore user agency through direct,
							encrypted file transfers. Built with Rust, React, and Iroh's P2P
							foundation.
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
							Open source •
							<a
								href="https://github.com/alDuncanson/ginseng"
								target="_blank"
								rel="noopener noreferrer"
							>
								View on GitHub
							</a>
						</p>
					</footer>
				</div>
			</main>
		</div>
	);
}

export default App;
