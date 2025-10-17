import "./App.css";
import { open } from "@tauri-apps/plugin-dialog";
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
		</main>
	);
}

export default App;
