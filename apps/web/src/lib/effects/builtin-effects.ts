export interface BuiltinEffectManifest {
	id: string;
	name: string;
	fileName: string;
	url: string;
	version: string;
	defaultBlendMode: string;
}

export const BUILTIN_EFFECTS_MANIFEST: BuiltinEffectManifest[] = [
	{
		id: "stars",
		name: "粒子星星",
		fileName: "stars.mp4",
		url: "/effects/stars.mp4",
		version: "h264-v2",
		defaultBlendMode: "screen",
	},
	{
		id: "leaves",
		name: "楓紅落葉",
		fileName: "leaves.mp4",
		url: "/effects/leaves.mp4",
		version: "h264-v2",
		defaultBlendMode: "screen",
	},
	{
		id: "clouds",
		name: "天空浮雲",
		fileName: "clouds.mp4",
		url: "/effects/clouds.mp4",
		version: "h264-v2",
		defaultBlendMode: "screen",
	},
];
