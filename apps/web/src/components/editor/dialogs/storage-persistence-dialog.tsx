"use client";

import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogBody,
	DialogContent,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { useStoragePersistence } from "@/hooks/use-storage-persistence";

export function StoragePersistenceDialog() {
	const { showDialog, onConfirm, onDismiss } = useStoragePersistence();

	return (
		<Dialog open={showDialog} onOpenChange={(open) => !open && onDismiss()}>
			<DialogContent className="sm:max-w-md">
				<DialogHeader>
					<DialogTitle>保護您的專案</DialogTitle>
				</DialogHeader>
				<DialogBody>
					<p className="text-base text-muted-foreground">
						您的瀏覽器可能在儲存空間不足時自動刪除您的專案。
					</p>
					<p className="text-base text-muted-foreground">
						允許 OpenCut 保護您的專案嗎？
					</p>
				</DialogBody>
				<DialogFooter>
					<Button variant="outline" onClick={onDismiss}>
						稍後再說
					</Button>
					<Button onClick={onConfirm}>允許</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
