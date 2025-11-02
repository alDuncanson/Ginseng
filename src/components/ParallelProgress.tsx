import { AlertCircle, Check, Clock, Download, Upload } from "lucide-react";
import { Progress } from "@/components/ui/progress";
import type { FileProgress, TransferProgress } from "@/types/progress";
import { calculateProgress, formatBytes, formatDuration } from "@/types/progress";

interface ParallelProgressProps {
	transfer: TransferProgress;
	compact?: boolean;
}

export function ParallelProgress({ transfer, compact = false }: ParallelProgressProps) {
	const overallProgress = calculateProgress(transfer.transferredBytes, transfer.totalBytes);

	const getStageDisplay = () => {
		switch (transfer.stage) {
			case "completed":
				return "COMPLETED";
			case "failed":
				return "FAILED";
			case "transferring":
				return "RUNNING";
			default:
				return "QUEUED";
		}
	};

	if (compact) {
		return (
			<div className="border border-foreground/20 p-4">
				<div className="flex items-center gap-3">
					{transfer.transferType === "upload" ? (
						<Upload className="h-4 w-4 text-muted-foreground" />
					) : (
						<Download className="h-4 w-4 text-muted-foreground" />
					)}
					<div className="flex-1 space-y-2">
						<div className="flex items-center justify-between text-sm">
							<span className="font-normal">
								{transfer.transferType === "upload" ? "uploading" : "downloading"}{" "}
								{transfer.totalFiles} file(s)
							</span>
							<span className="text-xs uppercase tracking-wider">{getStageDisplay()}</span>
						</div>
						{transfer.stage === "transferring" && (
							<>
								<Progress value={overallProgress} />
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
			</div>
		);
	}

	return (
		<div className="border border-foreground/20 p-6">
			<div className="space-y-6">
				<div className="flex items-center justify-between pb-3 border-b border-foreground/10">
					<div className="text-xs uppercase tracking-wider text-muted-foreground">
						{transfer.transferType === "upload" ? "Upload" : "Download"} Progress
					</div>
					<div className="text-xs uppercase tracking-wider">{getStageDisplay()}</div>
				</div>

				<div className="space-y-3">
					<div className="flex justify-between text-sm py-1">
						<span className="text-muted-foreground">Overall</span>
						<span>{overallProgress}%</span>
					</div>
					<Progress value={overallProgress} className="h-0.5" />
					<div className="flex justify-between text-xs text-muted-foreground pt-1">
						<span>
							{formatBytes(transfer.transferredBytes)} / {formatBytes(transfer.totalBytes)}
						</span>
						<span>
							{transfer.completedFiles} / {transfer.totalFiles} files
						</span>
					</div>
				</div>

				{(transfer.transferRate || transfer.etaSeconds) && (
					<div className="flex gap-8 text-sm pt-2">
						{transfer.transferRate && (
							<div className="flex gap-2">
								<span className="text-muted-foreground">Speed</span>
								<span>{formatBytes(transfer.transferRate)}/s</span>
							</div>
						)}
						{transfer.etaSeconds && (
							<div className="flex gap-2">
								<span className="text-muted-foreground">ETA</span>
								<span>{formatDuration(transfer.etaSeconds)}</span>
							</div>
						)}
					</div>
				)}

				{transfer.files.length > 0 && (
					<div className="space-y-3 pt-2">
						<div className="text-xs uppercase tracking-wider text-muted-foreground">
							Active Files · {transfer.completedFiles}/{transfer.totalFiles}
						</div>
						<div className="max-h-64 space-y-4 overflow-y-auto">
							{transfer.files.map((file) => (
								<FileProgressItem key={file.fileId} file={file} />
							))}
						</div>
					</div>
				)}

				{transfer.error && (
					<div className="border border-destructive p-4 text-sm">
						<div className="flex items-start gap-2">
							<AlertCircle className="mt-0.5 h-4 w-4 shrink-0 text-destructive" />
							<div>
								<div className="font-normal">Transfer Failed</div>
								<div className="text-muted-foreground text-xs mt-1">{transfer.error}</div>
							</div>
						</div>
					</div>
				)}
			</div>
		</div>
	);
}

function FileProgressItem({ file }: { file: FileProgress }) {
	const progress = calculateProgress(file.transferredBytes, file.totalBytes);

	const getIcon = () => {
		switch (file.status) {
			case "completed":
				return <Check className="h-3 w-3 text-muted-foreground" />;
			case "failed":
				return <AlertCircle className="h-3 w-3 text-destructive" />;
			case "transferring":
				return <Clock className="h-3 w-3 animate-pulse text-muted-foreground" />;
			default:
				return <Clock className="h-3 w-3 text-muted-foreground/50" />;
		}
	};

	const getStatusDisplay = () => {
		switch (file.status) {
			case "completed":
				return "100%";
			case "failed":
				return "FAILED";
			case "transferring":
				return `${progress}%`;
			default:
				return "0%";
		}
	};

	return (
		<div className="border border-foreground/10 p-4">
			<div className="mb-3 flex items-center justify-between">
				<div className="flex min-w-0 flex-1 items-center gap-2.5">
					{getIcon()}
					<span className="truncate text-sm" title={file.name}>
						{file.name}
					</span>
				</div>
				<div className="flex items-center gap-4">
					<span className="text-xs font-normal">{getStatusDisplay()}</span>
					<span className="text-xs text-muted-foreground">{formatBytes(file.totalBytes)}</span>
				</div>
			</div>

			{file.status === "transferring" && (
				<div className="space-y-2">
					<Progress value={progress} className="h-px" />
					<div className="flex justify-between text-xs text-muted-foreground pt-0.5">
						<span>{formatBytes(file.transferredBytes)}</span>
						<span>{formatBytes(file.totalBytes)}</span>
					</div>
				</div>
			)}

			{file.error && (
				<div className="mt-2 flex items-start gap-1.5 text-xs text-destructive pt-1">
					<AlertCircle className="mt-0.5 h-3 w-3 shrink-0" />
					<span>{file.error}</span>
				</div>
			)}
		</div>
	);
}
