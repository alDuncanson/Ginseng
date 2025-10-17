import "./App.css";
import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

type FileInfo = { path: string; exists: boolean; size?: number | null };
type ProcessFilesResponse = {
	total: number;
	processed: number;
	files: FileInfo[];
};

function App() {
	const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
	const [isDragOver, setIsDragOver] = useState(false);

	// Processing state for invoking the Rust command
	const [processing, setProcessing] = useState(false);
	const [result, setResult] = useState<ProcessFilesResponse | null>(null);
	const [procError, setProcError] = useState<string | null>(null);

	// Iroh send/download state
	const [sendBusy, setSendBusy] = useState(false);
	const [ticket, setTicket] = useState<string>("");
	const [recvTicket, setRecvTicket] = useState<string>("");
	const [destPath, setDestPath] = useState<string>("");
	const [downloadBusy, setDownloadBusy] = useState(false);
	const [downloadMsg, setDownloadMsg] = useState<string>("");

	const handleOpenFiles = async () => {
		try {
			const files = await open({
				multiple: true,
			});

			if (files) {
				const filePaths = Array.isArray(files) ? files : [files];
				setSelectedFiles(filePaths);
				console.log("Selected files:", filePaths);
			}
		} catch (error) {
			console.error("Error opening file dialog:", error);
		}
	};

	const handleDragOver = (e: React.DragEvent) => {
		e.preventDefault();
		setIsDragOver(true);
	};

	const handleDragLeave = (e: React.DragEvent) => {
		e.preventDefault();
		setIsDragOver(false);
	};

	const handleDrop = (e: React.DragEvent) => {
		e.preventDefault();
		setIsDragOver(false);

		// Extract file paths from dropped files
		const files = Array.from(e.dataTransfer.files);
		const filePaths = files.map(
			(file) => (file as File & { path: string }).path,
		); // Use file.path for full file paths in Tauri
		setSelectedFiles(filePaths);
		console.log("Dropped files:", filePaths);
	};

	const runProcessing = async () => {
		if (!selectedFiles.length) return;
		setProcessing(true);
		setProcError(null);
		setResult(null);
		try {
			const res = await invoke<ProcessFilesResponse>("process_files", {
				paths: selectedFiles,
			});
			setResult(res);
			console.log("process_files result:", res);
		} catch (err) {
			console.error("process_files error:", err);
			setProcError(String(err));
		} finally {
			setProcessing(false);
		}
	};

	const sendSelectedViaIroh = async () => {
		if (!selectedFiles.length) return;
		if (selectedFiles.length > 1) {
			alert("Please select a single file to send.");
			return;
		}
		setSendBusy(true);
		setTicket("");
		try {
			const t = await invoke<string>("iroh_send", {
				path: selectedFiles[0],
			});
			setTicket(t);
		} catch (e) {
			alert(`Send failed: ${e}`);
		} finally {
			setSendBusy(false);
		}
	};

	const chooseDest = async () => {
		try {
			const path = await save({
				title: "Save received file as…",
			});
			if (path) setDestPath(path);
		} catch (e) {
			console.error("save dialog error", e);
		}
	};

	const downloadViaIroh = async () => {
		if (!recvTicket) {
			alert("Enter a ticket to download.");
			return;
		}
		if (!destPath) {
			alert("Choose a destination path first.");
			return;
		}
		setDownloadBusy(true);
		setDownloadMsg("");
		try {
			await invoke("iroh_download", {
				ticket: recvTicket,
				destPath: destPath,
			});
			setDownloadMsg("Download complete.");
		} catch (e) {
			setDownloadMsg(`Download failed: ${e}`);
		} finally {
			setDownloadBusy(false);
		}
	};

	return (
		<main className="container">
			<div className="file-drop-zone">
				<button
					type="button"
					onClick={handleOpenFiles}
					onDragOver={handleDragOver}
					onDragLeave={handleDragLeave}
					onDrop={handleDrop}
					style={{
						width: "100%",
						height: "200px",
						border: `2px dashed ${isDragOver ? "#007ACC" : "#ccc"}`,
						borderRadius: "8px",
						padding: "20px",
						textAlign: "center",
						cursor: "pointer",
						backgroundColor: isDragOver ? "#f0f8ff" : "transparent",
						fontSize: "16px",
						transition: "all 0.2s ease",
					}}
				>
					{isDragOver
						? "Drop files here"
						: "Click to select files or drag & drop"}
				</button>
				{selectedFiles.length > 0 && (
					<div style={{ marginTop: "20px" }}>
						<h3>Selected Files:</h3>
						<ul>
							{selectedFiles.map((file) => (
								<li key={file}>{file}</li>
							))}
						</ul>
						<div style={{ display: "flex", gap: 8, marginTop: 8 }}>
							<button
								type="button"
								onClick={sendSelectedViaIroh}
								disabled={sendBusy || selectedFiles.length !== 1}
								style={{ padding: "8px 12px" }}
							>
								{sendBusy ? "Hashing…" : "Send via iroh"}
							</button>
							{ticket && (
								<div style={{ flex: 1, display: "flex", gap: 6 }}>
									<input
										value={ticket}
										readOnly
										style={{ flex: 1, fontFamily: "monospace" }}
									/>
									<button
										onClick={() => navigator.clipboard.writeText(ticket)}
										type="button"
									>
										Copy
									</button>
								</div>
							)}
						</div>
						<div style={{ marginTop: 12 }}>
							<button
								type="button"
								onClick={runProcessing}
								disabled={!selectedFiles.length || processing}
								style={{
									padding: "8px 12px",
									cursor: processing ? "wait" : "pointer",
								}}
							>
								{processing ? "Processing…" : "Process selected files"}
							</button>
						</div>

						{procError && (
							<p style={{ color: "crimson", marginTop: 8 }}>
								Error: {procError}
							</p>
						)}

						{result && (
							<div style={{ marginTop: 16 }}>
								<h4>Process Result</h4>
								<p>
									Total: {result.total}, Processed: {result.processed}
								</p>
								<ul>
									{result.files.map((f) => (
										<li key={f.path}>
											{f.path} —{" "}
											{f.exists
												? `exists${typeof f.size === "number" ? ` (${f.size} bytes)` : ""}`
												: "missing"}
										</li>
									))}
								</ul>
							</div>
						)}
					</div>
				)}
			</div>

			<div style={{ marginTop: 32 }}>
				<h3>Receive via iroh</h3>
				<div style={{ display: "flex", gap: 8 }}>
					<input
						placeholder="Paste ticket here"
						value={recvTicket}
						onChange={(e) => setRecvTicket(e.target.value)}
						style={{ flex: 2, fontFamily: "monospace" }}
					/>
					<input
						placeholder="Destination path"
						value={destPath}
						onChange={(e) => setDestPath(e.target.value)}
						style={{ flex: 2 }}
					/>
					<button type="button" onClick={chooseDest}>
						Choose…
					</button>
					<button
						type="button"
						onClick={downloadViaIroh}
						disabled={downloadBusy}
					>
						{downloadBusy ? "Downloading…" : "Download"}
					</button>
				</div>
				{downloadMsg && <p style={{ marginTop: 8 }}>{downloadMsg}</p>}
			</div>
		</main>
	);
}

export default App;
