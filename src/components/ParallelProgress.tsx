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
			<Card className="p-4">
				<div className="flex items-center gap-3">
					{transfer.transferType === "upload" ? (
						<Upload className="h-4 w-4" />
					) : (
						<Download className="h-4 w-4" />
					)}
					<div className="flex-1 space-y-2">
						<div className="flex items-center justify-between text-sm">
							<span>
								{transfer.transferType === "upload" ? "uploading" : "downloading"}{" "}
								{transfer.totalFiles} file(s)
							</span>
							<Badge variant="outline">{getStageDisplay()}</Badge>
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
			</Card>
		);
	}

	return (
		<Card className="p-6">
			<div className="space-y-6">
				<div className="flex items-center justify-between">
					<div className="text-sm text-muted-foreground">
						{transfer.transferType === "upload" ? "upload" : "download"} progress
					</div>
					<div className="text-sm">{getStageDisplay()}</div>
				</div>

				<div className="space-y-3">
					<div className="flex justify-between text-sm">
						<span className="text-muted-foreground">overall</span>
						<span>{overallProgress}%</span>
					</div>
					<Progress value={overallProgress} className="h-1" />
					<div className="flex justify-between text-xs text-muted-foreground">
						<span>
							{formatBytes(transfer.transferredBytes)} / {formatBytes(transfer.totalBytes)}
						</span>
						<span>
							{transfer.completedFiles} / {transfer.totalFiles} files
						</span>
					</div>
				</div>

				{(transfer.transferRate || transfer.etaSeconds) && (
					<div className="grid grid-cols-2 gap-4 text-sm">
						{transfer.transferRate && (
							<div>
								<span className="text-muted-foreground">speed </span>
								<span>{formatBytes(transfer.transferRate)}/s</span>
							</div>
						)}
						{transfer.etaSeconds && (
							<div>
								<span className="text-muted-foreground">eta </span>
								<span>{formatDuration(transfer.etaSeconds)}</span>
							</div>
						)}
					</div>
				)}

				{transfer.files.length > 0 && (
					<div className="space-y-3">
						<div className="text-sm text-muted-foreground">
							active files · {transfer.completedFiles}/{transfer.totalFiles}
						</div>
						<div className="max-h-64 space-y-3 overflow-y-auto">
							{transfer.files.map((file) => (
								<FileProgressItem key={file.fileId} file={file} />
							))}
						</div>
					</div>
				)}

				{transfer.error && (
					<div className="bg-destructive/10 p-4 text-sm">
						<div className="flex items-start gap-2">
							<AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
							<div>
								<div className="font-medium">Transfer Failed</div>
								<div className="text-muted-foreground">{transfer.error}</div>
							</div>
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
				return <Check className="h-3 w-3" />;
			case "failed":
				return <AlertCircle className="h-3 w-3 text-destructive" />;
			case "transferring":
				return <Clock className="h-3 w-3 animate-pulse" />;
			default:
				return <Clock className="h-3 w-3 opacity-50" />;
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
		<div className="bg-card p-4">
			<div className="mb-3 flex items-center justify-between">
				<div className="flex min-w-0 flex-1 items-center gap-2">
					{getIcon()}
					<span className="truncate text-sm" title={file.name}>
						{file.name}
					</span>
				</div>
				<div className="flex items-center gap-3">
					<span className="text-xs">{getStatusDisplay()}</span>
					<span className="text-xs text-muted-foreground">{formatBytes(file.totalBytes)}</span>
				</div>
			</div>

			{file.status === "transferring" && (
				<div className="space-y-2">
					<Progress value={progress} className="h-[2px]" />
					<div className="flex justify-between text-xs text-muted-foreground">
						<span>{formatBytes(file.transferredBytes)}</span>
						<span>{formatBytes(file.totalBytes)}</span>
					</div>
				</div>
			)}

			{file.error && (
				<div className="mt-2 flex items-start gap-1 text-xs text-destructive">
					<AlertCircle className="mt-0.5 h-3 w-3 shrink-0" />
					<span>{file.error}</span>
				</div>
			)}
		</div>
	);
}
