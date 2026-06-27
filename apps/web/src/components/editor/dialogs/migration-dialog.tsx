"use client";

import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { useEditor } from "@/hooks/use-editor";
import { Loader2 } from "lucide-react";

export function MigrationDialog() {
	const editor = useEditor();
	const migrationState = editor.project.getMigrationState();

	if (!migrationState.isMigrating) return null;

	const title = migrationState.projectName
		? "更新專案中"
		: "更新多個專案中";
	const description = migrationState.projectName
		? `正在將「${migrationState.projectName}」從 v${migrationState.fromVersion} 升級至 v${migrationState.toVersion}`
		: `正在將專案從 v${migrationState.fromVersion} 升級至 v${migrationState.toVersion}`;

	return (
		<Dialog open={true}>
			<DialogContent
				className="sm:max-w-md"
				onPointerDownOutside={(event) => event.preventDefault()}
				onEscapeKeyDown={(event) => event.preventDefault()}
			>
				<DialogHeader>
					<DialogTitle>{title}</DialogTitle>
					<DialogDescription>{description}</DialogDescription>
				</DialogHeader>

				<div className="flex items-center justify-center py-4">
					<Loader2 className="text-muted-foreground size-8 animate-spin" />
				</div>
			</DialogContent>
		</Dialog>
	);
}
