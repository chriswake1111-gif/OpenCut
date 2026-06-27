import type {
	KeybindingConfig,
	ShortcutKey,
} from "@/lib/actions/keybinding";
import type { TActionWithOptionalArgs } from "./types";

export type TActionCategory =
	| "播放"
	| "導航"
	| "編輯"
	| "選取"
	| "歷史記錄"
	| "時間軸"
	| "控制"
	| "素材";

export interface TActionBaseDefinition {
	description: string;
	category: TActionCategory;
	args?: Record<string, unknown>;
}

export interface TActionDefinition extends TActionBaseDefinition {
	defaultShortcuts?: readonly ShortcutKey[];
}

export const ACTIONS = {
	"toggle-play": {
		description: "播放/暫停",
		category: "播放",
	},
	"stop-playback": {
		description: "停止播放",
		category: "播放",
	},
	"seek-forward": {
		description: "快進 1 秒",
		category: "播放",
		args: { seconds: "number" },
	},
	"seek-backward": {
		description: "倒退 1 秒",
		category: "播放",
		args: { seconds: "number" },
	},
	"frame-step-forward": {
		description: "向前一格",
		category: "導航",
	},
	"frame-step-backward": {
		description: "向後一格",
		category: "導航",
	},
	"jump-forward": {
		description: "快進 5 秒",
		category: "導航",
		args: { seconds: "number" },
	},
	"jump-backward": {
		description: "倒退 5 秒",
		category: "導航",
		args: { seconds: "number" },
	},
	"goto-start": {
		description: "跳至時間軸開頭",
		category: "導航",
	},
	"goto-end": {
		description: "跳至時間軸結尾",
		category: "導航",
	},
	split: {
		description: "在播放頭處分割素材",
		category: "編輯",
	},
	"split-left": {
		description: "分割並移除左側",
		category: "編輯",
	},
	"split-right": {
		description: "分割並移除右側",
		category: "編輯",
	},
	"delete-selected": {
		description: "刪除已選取的素材",
		category: "編輯",
	},
	"copy-selected": {
		description: "複製已選取的素材",
		category: "編輯",
	},
	"paste-copied": {
		description: "在播放頭處貼上素材",
		category: "編輯",
	},
	"toggle-snapping": {
		description: "切換吸附功能",
		category: "編輯",
	},
	"toggle-ripple-editing": {
		description: "切換波紋編輯",
		category: "編輯",
	},
	"select-all": {
		description: "全選素材",
		category: "選取",
	},
	"cancel-interaction": {
		description: "取消目前操作",
		category: "控制",
	},
	"deselect-all": {
		description: "取消全選",
		category: "選取",
	},
	"duplicate-selected": {
		description: "複製已選取的素材",
		category: "選取",
	},
	"toggle-elements-muted-selected": {
		description: "靜音/取消靜音已選取的素材",
		category: "選取",
	},
	"toggle-elements-visibility-selected": {
		description: "顯示/隱藏已選取的素材",
		category: "選取",
	},
	"toggle-bookmark": {
		description: "切換播放頭處的書籤",
		category: "時間軸",
	},
	undo: {
		description: "復原",
		category: "歷史記錄",
	},
	redo: {
		description: "重做",
		category: "歷史記錄",
	},
	"remove-media-asset": {
		description: "移除素材",
		category: "素材",
		args: { projectId: "string", assetId: "string" },
	},
	"remove-media-assets": {
		description: "移除多個素材",
		category: "素材",
		args: { projectId: "string", assetIds: "string[]" },
	},
} as const satisfies Record<string, TActionBaseDefinition>;

export type TAction = keyof typeof ACTIONS;

const ACTION_DEFAULT_SHORTCUTS = {
	"toggle-play": ["space", "k"],
	"seek-forward": ["l"],
	"seek-backward": ["j"],
	"frame-step-forward": ["right"],
	"frame-step-backward": ["left"],
	"jump-forward": ["shift+right"],
	"jump-backward": ["shift+left"],
	"goto-start": ["home", "enter"],
	"goto-end": ["end"],
	split: ["s"],
	"split-left": ["q"],
	"split-right": ["w"],
	"delete-selected": ["backspace", "delete"],
	"copy-selected": ["ctrl+c"],
	"paste-copied": ["ctrl+v"],
	"toggle-snapping": ["n"],
	"select-all": ["ctrl+a"],
	"cancel-interaction": ["escape"],
	"duplicate-selected": ["ctrl+d"],
	undo: ["ctrl+z"],
	redo: ["ctrl+shift+z", "ctrl+y"],
} as const satisfies Partial<Record<TActionWithOptionalArgs, readonly ShortcutKey[]>>;

const ACTION_DEFAULT_SHORTCUTS_BY_ACTION: Partial<
	Record<TAction, readonly ShortcutKey[]>
> = ACTION_DEFAULT_SHORTCUTS;

export function getActionDefinition({
	action,
}: {
	action: TAction;
}): TActionDefinition {
	return {
		...ACTIONS[action],
		defaultShortcuts: ACTION_DEFAULT_SHORTCUTS_BY_ACTION[action],
	};
}

export function getDefaultShortcuts(): KeybindingConfig {
	const shortcuts: KeybindingConfig = {};

	for (const [action, defaultShortcuts] of Object.entries(
		ACTION_DEFAULT_SHORTCUTS,
	) as Array<[TActionWithOptionalArgs, readonly ShortcutKey[]]>) {
		for (const shortcut of defaultShortcuts) {
			shortcuts[shortcut] = action;
		}
	}

	return shortcuts;
}
