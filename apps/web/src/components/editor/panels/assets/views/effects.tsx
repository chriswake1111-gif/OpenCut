"use client";

import { useEffect, useRef, useCallback, useState } from "react";
import { PanelView } from "@/components/editor/panels/assets/views/base-panel";
import { DraggableItem } from "@/components/editor/panels/assets/draggable-item";
import { effectsRegistry, EFFECT_TARGET_ELEMENT_TYPES } from "@/lib/effects";
import { effectPreviewService } from "@/services/renderer/effect-preview";
import { useEditor } from "@/hooks/use-editor";
import { buildEffectElement } from "@/lib/timeline/element-utils";
import type { EffectDefinition } from "@/lib/effects/types";

// Dynamic video effects imports
import { Sparkles, Leaf, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { processMediaAssets } from "@/lib/media/processing";
import { buildElementFromMedia } from "@/lib/timeline/element-utils";

interface DynamicVideoEffect {
	id: string;
	name: string;
	fileName: string;
	url: string;
	icon: "sparkles" | "leaf";
	gradient: string;
}

const DYNAMIC_VIDEO_EFFECTS: DynamicVideoEffect[] = [
	{
		id: "stars",
		name: "粒子星星",
		fileName: "stars.mp4",
		url: "/effects/stars.mp4",
		icon: "sparkles",
		gradient: "from-blue-950 via-indigo-900 to-slate-900 text-indigo-300",
	},
	{
		id: "leaves",
		name: "楓紅落葉",
		fileName: "leaves.mp4",
		url: "/effects/leaves.mp4",
		icon: "leaf",
		gradient: "from-amber-950 via-red-950 to-stone-900 text-red-400",
	},
];

export function EffectsView() {
	const effects = effectsRegistry.getAll();

	return (
		<PanelView title="特效">
			<div className="flex flex-col gap-6 p-4 h-full overflow-y-auto scrollbar-thin">
				<div className="flex flex-col gap-2.5">
					<h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider select-none">
						畫面濾鏡 (Shader Filters)
					</h3>
					<EffectsGrid effects={effects} />
				</div>

				<div className="flex flex-col gap-2.5">
					<h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider select-none">
						動態粒子特效 (Dynamic Particle Overlays)
					</h3>
					<DynamicEffectsGrid />
				</div>
			</div>
		</PanelView>
	);
}

function EffectsGrid({ effects }: { effects: EffectDefinition[] }) {
	return (
		<div
			className="grid gap-2"
			style={{ gridTemplateColumns: "repeat(auto-fill, minmax(96px, 1fr))" }}
		>
			{effects.map((effect) => (
				<EffectItem key={effect.type} effect={effect} />
			))}
		</div>
	);
}

function DynamicEffectsGrid() {
	return (
		<div
			className="grid gap-2"
			style={{ gridTemplateColumns: "repeat(auto-fill, minmax(96px, 1fr))" }}
		>
			{DYNAMIC_VIDEO_EFFECTS.map((effect) => (
				<DynamicEffectItem key={effect.id} effect={effect} />
			))}
		</div>
	);
}

function DynamicEffectItem({ effect }: { effect: DynamicVideoEffect }) {
	const editor = useEditor();
	const activeProject = useEditor((e) => e.project.getActive());
	const [isLoading, setIsLoading] = useState(false);

	const handleApplyEffect = async () => {
		if (!activeProject) {
			toast.error("沒有作用中的專案");
			return;
		}

		setIsLoading(true);
		try {
			// 1. Check if the asset has already been imported
			const existingAssets = editor.media.getAssets();
			let targetAsset = existingAssets.find((a) => a.name === effect.fileName);

			if (!targetAsset) {
				// 2. Fetch the video file as blob
				const response = await fetch(effect.url);
				if (!response.ok) {
					throw new Error(`無法下載特效檔案: ${response.statusText}`);
				}
				const blob = await response.blob();
				const file = new File([blob], effect.fileName, { type: "video/mp4" });

				// 3. Process it to generate thumbnails & details
				const processed = await processMediaAssets({ files: [file] });
				if (processed.length === 0) {
					throw new Error("處理特效影片失敗");
				}

				const asset = processed[0];
				// 4. Register in project media DB
				const addedAsset = await editor.media.addMediaAsset({
					projectId: activeProject.metadata.id,
					asset,
				});
				if (!addedAsset) {
					throw new Error("儲存特效素材失敗");
				}
				targetAsset = addedAsset;
			}

			// 5. Build timeline element and insert it
			const duration = targetAsset.duration ?? 10.0;
			const element = buildElementFromMedia({
				mediaId: targetAsset.id,
				mediaType: "video",
				name: effect.name,
				duration,
				startTime: editor.playback.getCurrentTime(),
			});

			// Force blendMode to screen to make it transparent overlay
			if (element.type === "video") {
				element.blendMode = "screen";
			}

			editor.timeline.insertElement({
				element,
				placement: { mode: "auto" },
			});

			toast.success(`已成功套用「${effect.name}」`);
		} catch (error) {
			console.error("Apply dynamic effect failed:", error);
			toast.error(error instanceof Error ? error.message : "套用特效失敗，請稍後再試");
		} finally {
			setIsLoading(false);
		}
	};

	const Icon = effect.icon === "sparkles" ? Sparkles : Leaf;

	return (
		<div className="flex flex-col gap-1 items-center">
			<button
				type="button"
				onClick={handleApplyEffect}
				disabled={isLoading}
				className={`relative group flex size-24 items-center justify-center rounded-xl bg-gradient-to-br ${effect.gradient} border border-white/5 hover:border-white/10 transition-all hover:scale-105 duration-200 active:scale-95 shadow-md shadow-black/40 hover:shadow-indigo-500/10 cursor-pointer overflow-hidden`}
			>
				{/* Glowing backdrop */}
				<div className="absolute inset-0 bg-white/5 opacity-0 group-hover:opacity-100 transition-opacity duration-300" />
				
				{isLoading ? (
					<Loader2 className="size-8 animate-spin text-white/80" />
				) : (
					<Icon className="size-8 transition-transform group-hover:rotate-12 duration-300" />
				)}
			</button>
			<span className="text-[11px] font-medium text-muted-foreground truncate w-24 text-center mt-1 select-none">
				{effect.name}
			</span>
		</div>
	);
}

function EffectPreviewCanvas({ effectType }: { effectType: string }) {
	const canvasRef = useRef<HTMLCanvasElement>(null);

	useEffect(() => {
		const render = () => {
			if (canvasRef.current) {
				effectPreviewService.renderPreview({
					effectType,
					params: {},
					targetCanvas: canvasRef.current,
				});
			}
		};

		render();
		return effectPreviewService.onPreviewImageReady({ callback: render });
	}, [effectType]);

	return <canvas ref={canvasRef} className="size-full" />;
}

function EffectItem({ effect }: { effect: EffectDefinition }) {
	const editor = useEditor();

	const handleAddToTimeline = useCallback(() => {
		const currentTime = editor.playback.getCurrentTime();
		const element = buildEffectElement({
			effectType: effect.type,
			startTime: currentTime,
		});

		editor.timeline.insertElement({
			placement: { mode: "auto", trackType: "effect" },
			element,
		});
	}, [editor, effect.type]);

	const preview = <EffectPreviewCanvas effectType={effect.type} />;

	return (
		<DraggableItem
			name={effect.name}
			preview={preview}
			dragData={{
				id: effect.type,
				name: effect.name,
				type: "effect",
				effectType: effect.type,
				targetElementTypes: EFFECT_TARGET_ELEMENT_TYPES,
			}}
			onAddToTimeline={handleAddToTimeline}
			aspectRatio={1}
			isRounded
			variant="card"
			containerClassName="w-full"
		/>
	);
}
