import { Channel, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Copy, File, Files, Folder, X } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { ParallelProgress } from "@/components/ParallelProgress";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { ProgressEvent, TransferProgress } from "@/types/progress";

interface FileInfo {
	name: string;
	relative_path: string;
	size: number;
}

interface ShareMetadata {
	files: FileInfo[];
	share_type: "SingleFile" | "MultipleFiles" | { Directory: { name: string } };
	total_size: number;
}

interface DownloadResult {
	metadata: ShareMetadata;
	download_path: string;
}

export function FileTransfer() {
	const [selectedPaths, setSelectedPaths] = useState<string[]>([]);
	const [ticket, setTicket] = useState<string>("");
	const [sendLoading, setSendLoading] = useState(false);

	const [receiveTicket, setReceiveTicket] = useState<string>("");
	const [receiveLoading, setReceiveLoading] = useState(false);
	const [lastDownload, setLastDownload] = useState<DownloadResult | null>(null);

	const [uploadProgress, setUploadProgress] = useState<TransferProgress | null>(null);
	const [downloadProgress, setDownloadProgress] = useState<TransferProgress | null>(null);

	const selectFiles = async () => {
		try {
			const files = await open({
				multiple: true,
				directory: false,
			});
			if (files) {
				const fileArray = Array.isArray(files) ? files : [files];
				setSelectedPaths(fileArray as string[]);
				setTicket("");
			}
		} catch {
			toast.error("Failed to select files");
		}
	};

	const selectFolder = async () => {
		try {
			const folder = await open({
				multiple: false,
				directory: true,
			});
			if (folder) {
				setSelectedPaths([folder as string]);
				setTicket("");
			}
		} catch {
			toast.error("Failed to select folder");
		}
	};

	const removeFile = (pathToRemove: string) => {
		const newPaths = selectedPaths.filter((path) => path !== pathToRemove);
		setSelectedPaths(newPaths);
		if (newPaths.length === 0) {
			setTicket("");
		}
	};

	const sendFiles = async () => {
		if (selectedPaths.length === 0) {
			toast.error("Please select files or a folder first");
			return;
		}

		const channel = new Channel<ProgressEvent>();
		let generatedTicket = "";

		channel.onmessage = (event: ProgressEvent) => {
			switch (event.event) {
				case "transferStarted":
				case "transferProgress":
					setUploadProgress(event.data.transfer);
					break;
				case "transferCompleted":
					setUploadProgress(event.data.transfer);
					if (generatedTicket) {
						setTicket(generatedTicket);
						toast.success("Share ticket generated!");
					}
					break;
				case "transferFailed":
					setUploadProgress(event.data.transfer);
					toast.error(`Failed: ${event.data.error}`);
					break;
			}
		};

		setSendLoading(true);
		setUploadProgress(null);

		try {
			generatedTicket = await invoke<string>("share_files_parallel", {
				channel,
				paths: selectedPaths,
			});
			setTicket(generatedTicket);
		} catch (error) {
			toast.error(`Failed to share files: ${error}`);
			setUploadProgress(null);
		} finally {
			setSendLoading(false);
		}
	};

	const copyTicket = async () => {
		try {
			await navigator.clipboard.writeText(ticket);
			toast.success("Ticket copied to clipboard");
		} catch {
			toast.error("Failed to copy ticket");
		}
	};

	const receiveFiles = async () => {
		if (!receiveTicket) {
			toast.error("Please enter a ticket");
			return;
		}

		const channel = new Channel<ProgressEvent>();

		channel.onmessage = (event: ProgressEvent) => {
			switch (event.event) {
				case "transferStarted":
				case "transferProgress":
					setDownloadProgress(event.data.transfer);
					break;
				case "transferCompleted":
					setDownloadProgress(event.data.transfer);
					toast.success("Files downloaded successfully!");
					break;
				case "transferFailed":
					setDownloadProgress(event.data.transfer);
					toast.error(`Failed: ${event.data.error}`);
					break;
			}
		};

		setReceiveLoading(true);
		setDownloadProgress(null);

		try {
			const result = await invoke<DownloadResult>("download_files_parallel", {
				channel,
				ticket: receiveTicket,
			});
			setLastDownload(result);
			setReceiveTicket("");
		} catch (error) {
			toast.error(`Failed to download files: ${error}`);
			setDownloadProgress(null);
		} finally {
			setReceiveLoading(false);
		}
	};

	const getFileName = (path: string) => {
		return path.split("/").pop() || path.split("\\").pop() || path;
	};

	const formatFileSize = (bytes: number): string => {
		if (bytes === 0) return "0 B";
		const k = 1024;
		const sizes = ["B", "KB", "MB", "GB", "TB"];
		const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
		const size = bytes / k ** i;
		return `${size.toFixed(2)} ${sizes[i]}`;
	};

	const getShareTypeDisplay = (shareType: ShareMetadata["share_type"]) => {
		if (shareType === "SingleFile") return "Single File";
		if (shareType === "MultipleFiles") return "Multiple Files";
		if (typeof shareType === "object" && "Directory" in shareType) {
			return `Directory: ${shareType.Directory.name}`;
		}
		return "Unknown";
	};

	return (
		<div className="min-h-screen p-12">
			<div className="max-w-3xl mx-auto">
				<div className="flex items-baseline justify-between mb-16 pb-4 border-b border-foreground/20">
					<h1 className="text-5xl font-light tracking-wide">GINSENG</h1>
					<div className="text-xs tracking-wider uppercase text-muted-foreground">
						{new Date().toLocaleTimeString("en-US", { hour12: false })} UTC
					</div>
				</div>

				<Tabs defaultValue="send" className="w-full">
					<TabsList className="mb-12">
						<TabsTrigger value="send">send</TabsTrigger>
						<TabsTrigger value="receive">receive</TabsTrigger>
					</TabsList>

					<TabsContent value="send" className="space-y-8">
						<div className="grid grid-cols-2 gap-3">
							<Button
								variant="outline"
								onClick={selectFiles}
								className="justify-start h-auto py-3 border"
							>
								<Files className="h-3.5 w-3.5 mr-2" />
								<span className="font-normal">select files</span>
							</Button>
							<Button
								variant="outline"
								onClick={selectFolder}
								className="justify-start h-auto py-3 border"
							>
								<Folder className="h-3.5 w-3.5 mr-2" />
								<span className="font-normal">select folder</span>
							</Button>
						</div>

						{selectedPaths.length > 0 && (
							<div className="space-y-4 pt-2">
								<div className="text-xs uppercase tracking-wider text-muted-foreground">
									Selected · {selectedPaths.length} {selectedPaths.length === 1 ? "item" : "items"}
								</div>
								<div className="space-y-0 border-t border-foreground/10">
									{selectedPaths.map((path) => (
										<div
											key={path}
											className="flex items-center justify-between py-2.5 border-b border-foreground/10 last:border-0"
										>
											<div className="flex items-center gap-3 flex-1 min-w-0">
												<File className="h-3 w-3 flex-shrink-0 text-muted-foreground" />
												<span className="text-sm truncate" title={path}>
													{getFileName(path)}
												</span>
											</div>
											<Button
												variant="ghost"
												size="sm"
												onClick={() => removeFile(path)}
												className="h-7 w-7 p-0 flex-shrink-0 hover:bg-transparent"
											>
												<X className="h-3.5 w-3.5" />
											</Button>
										</div>
									))}
								</div>
							</div>
						)}

						<Button
							onClick={sendFiles}
							disabled={selectedPaths.length === 0 || sendLoading}
							className="w-full h-11 font-normal"
						>
							{sendLoading ? "generating ticket..." : "generate share ticket"}
						</Button>

						{uploadProgress && <ParallelProgress transfer={uploadProgress} compact={false} />}

						{ticket && (
							<div className="border border-foreground/20 p-6">
								<div className="space-y-4">
									<div className="text-xs uppercase tracking-wider text-muted-foreground">
										Share Ticket
									</div>
									<div className="flex gap-2">
										<Input value={ticket} readOnly className="text-xs border" />
										<Button variant="outline" size="icon" onClick={copyTicket} className="border">
											<Copy className="h-4 w-4" />
										</Button>
									</div>
								</div>
							</div>
						)}
					</TabsContent>

					<TabsContent value="receive" className="space-y-8">
						<div className="space-y-3">
							<Label className="text-xs uppercase tracking-wider text-muted-foreground">
								Ticket
							</Label>
							<Input
								placeholder="paste share ticket here..."
								value={receiveTicket}
								onChange={(e) => setReceiveTicket(e.target.value)}
								className="text-xs border"
							/>
						</div>

						<Button
							onClick={receiveFiles}
							disabled={!receiveTicket || receiveLoading}
							className="w-full h-11 font-normal"
						>
							{receiveLoading ? "downloading..." : "download files"}
						</Button>

						{downloadProgress && <ParallelProgress transfer={downloadProgress} compact={false} />}

						{lastDownload && (
							<div className="border border-foreground/20 p-6">
								<div className="space-y-6">
									<div className="text-xs uppercase tracking-wider text-muted-foreground">
										Last Download
									</div>
									<div className="space-y-3">
										<div className="flex justify-between text-sm py-1.5 border-b border-foreground/10">
											<span className="text-muted-foreground">Type</span>
											<span>{getShareTypeDisplay(lastDownload.metadata.share_type)}</span>
										</div>

										<div className="flex justify-between text-sm py-1.5 border-b border-foreground/10">
											<span className="text-muted-foreground">Files</span>
											<span>{lastDownload.metadata.files.length}</span>
										</div>

										<div className="flex justify-between text-sm py-1.5 border-b border-foreground/10">
											<span className="text-muted-foreground">Size</span>
											<span>{formatFileSize(lastDownload.metadata.total_size)}</span>
										</div>

										<div className="flex justify-between text-sm py-1.5">
											<span className="text-muted-foreground">Location</span>
											<span className="truncate ml-4 text-right" title={lastDownload.download_path}>
												{lastDownload.download_path}
											</span>
										</div>
									</div>

									{lastDownload.metadata.files.length > 0 && (
										<div className="pt-2 space-y-3">
											<div className="text-xs uppercase tracking-wider text-muted-foreground">
												Files
											</div>
											<div className="max-h-40 overflow-y-auto space-y-0 border-t border-foreground/10">
												{lastDownload.metadata.files.map((file) => (
													<div
														key={file.relative_path}
														className="flex items-center justify-between text-xs py-2 border-b border-foreground/10 last:border-0"
													>
														<span className="truncate" title={file.relative_path}>
															{file.relative_path}
														</span>
														<span className="text-muted-foreground ml-4">
															{formatFileSize(file.size)}
														</span>
													</div>
												))}
											</div>
										</div>
									)}
								</div>
							</div>
						)}
					</TabsContent>
				</Tabs>
			</div>
		</div>
	);
}
