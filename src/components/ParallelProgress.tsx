import { AlertCircle, Check, Clock, Download, Upload } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import type { FileProgress, TransferProgress } from "@/types/progress";
import { calculateProgress, formatBytes, formatDuration } from "@/types/progress";

interface ParallelProgressProps {
	transfer: TransferProgress;
	compact?: boolean;
}

export function ParallelProgress({ transfer, compact = false }: ParallelProgressProps) {
	const overallProgress = calculateProgress(transfer.transferredBytes, transfer.totalBytes);

	const getStageColor = () => {
		switch (transfer.stage) {
			case "completed":
				return "bg-green-500/10 text-green-500";
			case "failed":
				return "bg-red-500/10 text-red-500";
			case "transferring":
				return "bg-blue-500/10 text-blue-500";
			default:
				return "bg-gray-500/10 text-gray-500";
		}
	};

	if (compact) {
		return (
			<Card className="p-3">
				<div className="flex items-center gap-3">
					{transfer.transferType === "upload" ? (
						<Upload className="h-4 w-4 text-muted-foreground" />
					) : (
						<Download className="h-4 w-4 text-muted-foreground" />
					)}
					<div className="flex-1 space-y-2">
						<div className="flex items-center justify-between text-sm">
							<span className="font-medium">
								{transfer.transferType === "upload" ? "Uploading" : "Downloading"}{" "}
								{transfer.totalFiles} file(s)
							</span>
							<Badge variant="outline" className={getStageColor()}>
								{transfer.stage}
							</Badge>
						</div>
						{transfer.stage === "transferring" && (
							<>
								<Progress value={overallProgress} className="h-2" />
								<div className="flex justify-between text-xs text-muted-foreground">
									<span>
										{formatBytes(transfer.transferredBytes)} / {formatBytes(transfer.totalBytes)}
									</span>
									<span>{overallProgress}%</span>
								</div>
							</>
						)}
					</div>
				</div>
			</Card>
		);
	}

	return (
		<Card className="p-4">
			<div className="space-y-4">
				{/* Header */}
				<div className="flex items-center justify-between">
					<div className="flex items-center gap-2">
						{transfer.transferType === "upload" ? (
							<Upload className="h-5 w-5" />
						) : (
							<Download className="h-5 w-5" />
						)}
						<h3 className="font-semibold">
							{transfer.transferType === "upload" ? "Upload" : "Download"} Progress
						</h3>
					</div>
					<Badge variant="outline" className={getStageColor()}>
						{transfer.stage}
					</Badge>
				</div>

				{/* Overall Progress */}
				<div className="space-y-2">
					<div className="flex justify-between text-sm">
						<span>Overall Progress</span>
						<span className="font-medium">{overallProgress}%</span>
					</div>
					<Progress value={overallProgress} className="h-3" />
					<div className="flex justify-between text-xs text-muted-foreground">
						<span>
							{formatBytes(transfer.transferredBytes)} / {formatBytes(transfer.totalBytes)}
						</span>
						<span>
							{transfer.completedFiles} / {transfer.totalFiles} files
						</span>
					</div>
				</div>

				{/* Stats */}
				{(transfer.transferRate || transfer.etaSeconds) && (
					<div className="grid grid-cols-2 gap-4 text-sm">
						{transfer.transferRate && (
							<div>
								<span className="text-muted-foreground">Speed: </span>
								<span className="font-medium">{formatBytes(transfer.transferRate)}/s</span>
							</div>
						)}
						{transfer.etaSeconds && (
							<div>
								<span className="text-muted-foreground">ETA: </span>
								<span className="font-medium">{formatDuration(transfer.etaSeconds)}</span>
							</div>
						)}
					</div>
				)}

				{/* Files */}
				{transfer.files.length > 0 && (
					<div className="space-y-2">
						<h4 className="text-sm font-medium">
							Files ({transfer.completedFiles}/{transfer.totalFiles})
						</h4>
						<div className="max-h-64 space-y-2 overflow-y-auto">
							{transfer.files.map((file) => (
								<FileProgressItem key={file.fileId} file={file} />
							))}
						</div>
					</div>
				)}

				{/* Error */}
				{transfer.error && (
					<div className="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-800">
						<AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
						<div>
							<div className="font-medium">Transfer Failed</div>
							<div>{transfer.error}</div>
						</div>
					</div>
				)}
			</div>
		</Card>
	);
}

function FileProgressItem({ file }: { file: FileProgress }) {
	const progress = calculateProgress(file.transferredBytes, file.totalBytes);

	const getIcon = () => {
		switch (file.status) {
			case "completed":
				return <Check className="h-3 w-3 text-green-500" />;
			case "failed":
				return <AlertCircle className="h-3 w-3 text-red-500" />;
			case "transferring":
				return <Clock className="h-3 w-3 animate-pulse text-blue-500" />;
			default:
				return <Clock className="h-3 w-3 text-gray-400" />;
		}
	};

	const getStatusColor = () => {
		switch (file.status) {
			case "completed":
				return "bg-green-500/10 text-green-600";
			case "failed":
				return "bg-red-500/10 text-red-600";
			case "transferring":
				return "bg-blue-500/10 text-blue-600";
			default:
				return "bg-gray-500/10 text-gray-600";
		}
	};

	return (
		<div className="rounded-md border bg-muted/30 p-3">
			<div className="mb-2 flex items-center justify-between">
				<div className="flex min-w-0 flex-1 items-center gap-2">
					{getIcon()}
					<span className="truncate text-sm font-medium" title={file.name}>
						{file.name}
					</span>
				</div>
				<div className="flex items-center gap-2">
					<Badge variant="outline" className={`text-xs ${getStatusColor()}`}>
						{file.status}
					</Badge>
					<span className="text-xs text-muted-foreground">{formatBytes(file.totalBytes)}</span>
				</div>
			</div>

			{file.status === "transferring" && (
				<div className="space-y-1">
					<Progress value={progress} className="h-1" />
					<div className="flex justify-between text-xs text-muted-foreground">
						<span>
							{formatBytes(file.transferredBytes)} / {formatBytes(file.totalBytes)}
						</span>
						<span>{progress}%</span>
					</div>
				</div>
			)}

			{file.error && (
				<div className="mt-2 flex items-start gap-1 text-xs text-red-600">
					<AlertCircle className="mt-0.5 h-3 w-3 shrink-0" />
					<span>{file.error}</span>
				</div>
			)}
		</div>
	);
}
