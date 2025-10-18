import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Send, Download, Copy, File } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

export function FileTransfer() {
	const [selectedFile, setSelectedFile] = useState<string>("");
	const [ticket, setTicket] = useState<string>("");
	const [sendLoading, setSendLoading] = useState(false);

	const [receiveTicket, setReceiveTicket] = useState<string>("");
	const [savePath, setSavePath] = useState<string>("");
	const [receiveLoading, setReceiveLoading] = useState(false);

	const selectFile = async () => {
		try {
			const file = await open({
				multiple: false,
			});
			if (file) {
				setSelectedFile(file as string);
				setTicket(""); // Clear previous ticket
			}
		} catch (error) {
			toast.error("Failed to select file");
		}
	};

	const sendFile = async () => {
		if (!selectedFile) {
			toast.error("Please select a file first");
			return;
		}

		setSendLoading(true);
		try {
			const generatedTicket = await invoke<string>("iroh_send", {
				path: selectedFile,
			});
			setTicket(generatedTicket);
			toast.success("Share ticket generated!");
		} catch (error) {
			toast.error(`Failed to send file: ${error}`);
		} finally {
			setSendLoading(false);
		}
	};

	const copyTicket = async () => {
		try {
			await navigator.clipboard.writeText(ticket);
			toast.success("Ticket copied to clipboard");
		} catch (error) {
			toast.error("Failed to copy ticket");
		}
	};

	const chooseSaveLocation = async () => {
		try {
			const path = await save({
				title: "Choose save location",
			});
			if (path) {
				setSavePath(path);
			}
		} catch (error) {
			toast.error("Failed to choose save location");
		}
	};

	const receiveFile = async () => {
		if (!receiveTicket) {
			toast.error("Please enter a ticket");
			return;
		}
		if (!savePath) {
			toast.error("Please choose save location");
			return;
		}

		setReceiveLoading(true);
		try {
			await invoke("iroh_download", {
				ticket: receiveTicket,
				destPath: savePath,
			});
			toast.success("File downloaded successfully!");
			setReceiveTicket("");
			setSavePath("");
		} catch (error) {
			toast.error(`Failed to download file: ${error}`);
		} finally {
			setReceiveLoading(false);
		}
	};

	const getFileName = (path: string) => {
		return path.split("/").pop() || path.split("\\").pop() || path;
	};

	return (
		<div className="max-w-2xl mx-auto p-6">
			<div className="text-center mb-8">
				<h1 className="text-3xl font-bold mb-2">Ginseng</h1>
				<p className="text-muted-foreground">
					Secure peer-to-peer file sharing
				</p>
			</div>

			<Tabs defaultValue="send" className="w-full">
				<TabsList className="grid w-full grid-cols-2">
					<TabsTrigger value="send">Send</TabsTrigger>
					<TabsTrigger value="receive">Receive</TabsTrigger>
				</TabsList>

				<TabsContent value="send">
					<Card>
						<CardHeader>
							<CardTitle className="flex items-center gap-2">
								<Send className="h-5 w-5" />
								Send File
							</CardTitle>
						</CardHeader>
						<CardContent className="space-y-4">
							<div className="space-y-2">
								<Label>Select File</Label>
								<div className="flex gap-2">
									<Button
										variant="outline"
										onClick={selectFile}
										className="w-full justify-start"
									>
										<File className="h-4 w-4 mr-2" />
										{selectedFile
											? getFileName(selectedFile)
											: "Choose file..."}
									</Button>
								</div>
							</div>

							<Button
								onClick={sendFile}
								disabled={!selectedFile || sendLoading}
								className="w-full"
							>
								{sendLoading ? "Generating..." : "Generate Ticket"}
							</Button>

							{ticket && (
								<div className="space-y-2">
									<Label>Share Ticket</Label>
									<div className="flex gap-2">
										<Input
											value={ticket}
											readOnly
											className="font-mono text-xs"
										/>
										<Button variant="outline" size="icon" onClick={copyTicket}>
											<Copy className="h-4 w-4" />
										</Button>
									</div>
									<p className="text-sm text-muted-foreground">
										Copy this ticket and send it to the receiver
									</p>
								</div>
							)}
						</CardContent>
					</Card>
				</TabsContent>

				<TabsContent value="receive">
					<Card>
						<CardHeader>
							<CardTitle className="flex items-center gap-2">
								<Download className="h-5 w-5" />
								Receive File
							</CardTitle>
						</CardHeader>
						<CardContent className="space-y-4">
							<div className="space-y-2">
								<Label>Ticket</Label>
								<Input
									placeholder="Paste the ticket here..."
									value={receiveTicket}
									onChange={(e) => setReceiveTicket(e.target.value)}
									className="font-mono text-xs"
								/>
							</div>

							<div className="space-y-2">
								<Label>Save Location</Label>
								<div className="flex gap-2">
									<Input
										placeholder="Choose where to save..."
										value={savePath}
										readOnly
									/>
									<Button variant="outline" onClick={chooseSaveLocation}>
										Browse
									</Button>
								</div>
							</div>

							<Button
								onClick={receiveFile}
								disabled={!receiveTicket || !savePath || receiveLoading}
								className="w-full"
							>
								{receiveLoading ? "Downloading..." : "Download"}
							</Button>
						</CardContent>
					</Card>
				</TabsContent>
			</Tabs>
		</div>
	);
}
