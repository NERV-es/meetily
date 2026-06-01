'use client';

import React, { useState } from 'react';
import { ChevronDown, Mic, Lock } from 'lucide-react';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Button } from '@/components/ui/button';
import { useConfig } from '@/contexts/ConfigContext';

interface TranscriptModelProps {
    provider: 'localWhisper' | 'parakeet' | 'deepgram' | 'elevenLabs' | 'groq' | 'openai' | 'assemblyai' | 'gemini' | 'cartesia' | 'speechmatics';
    model: string;
    apiKey?: string | null;
}

const LOCAL_PROVIDERS = ['localWhisper', 'parakeet'] as const;

const PROVIDER_MODELS: Record<string, string[]> = {
    localWhisper: [],
    parakeet: [],
    deepgram: ['nova-2', 'nova-2-phonecall', 'nova-2-meeting', 'nova-2-general'],
    groq: ['whisper-large-v3-turbo', 'whisper-large-v3', 'distil-whisper-large-v3-en'],
    openai: ['whisper-1', 'gpt-4o-transcribe', 'gpt-4o-mini-transcribe'],
    assemblyai: ['best', 'nano'],
    gemini: ['gemini-2.0-flash'],
    speechmatics: ['en', 'multilingual'],
};

const DEFAULT_MODELS: Record<string, string> = {
    deepgram: 'nova-2',
    groq: 'whisper-large-v3-turbo',
    openai: 'whisper-1',
    assemblyai: 'best',
    gemini: 'gemini-2.0-flash',
    speechmatics: 'en',
};

const PROVIDER_LABELS: Record<string, string> = {
    localWhisper: 'Local Whisper',
    parakeet: 'Parakeet',
    deepgram: 'Deepgram',
    groq: 'Groq',
    openai: 'OpenAI',
    assemblyai: 'AssemblyAI',
    gemini: 'Gemini',
    speechmatics: 'Speechmatics',
};

interface ModelQuickSwitcherProps {
    isRecording: boolean;
}

export function ModelQuickSwitcher({ isRecording }: ModelQuickSwitcherProps) {
    const { transcriptModelConfig, setTranscriptModelConfig } = useConfig();
    const [open, setOpen] = useState(false);
    const [expandedProvider, setExpandedProvider] = useState<string | null>(null);

    if (isRecording) return null;

    const currentProvider = transcriptModelConfig?.provider ?? 'localWhisper';
    const currentModel = transcriptModelConfig?.model ?? '';
    const isLocal = LOCAL_PROVIDERS.includes(currentProvider as any);

    const displayLabel = PROVIDER_LABELS[currentProvider] ?? currentProvider;
    const displayModel = isLocal ? '' : currentModel;

    function isCloudProviderLocked(provider: string): boolean {
        if (LOCAL_PROVIDERS.includes(provider as any)) return false;
        // If current provider matches, use current apiKey
        if (provider === currentProvider) {
            return !transcriptModelConfig?.apiKey;
        }
        // For other providers we don't have the key stored, assume not locked
        // (user can configure from settings if needed)
        return false;
    }

    function handleProviderClick(provider: string) {
        const isLocalProvider = LOCAL_PROVIDERS.includes(provider as any);
        const models = PROVIDER_MODELS[provider] ?? [];

        if (isLocalProvider || models.length === 0) {
            // Switch immediately, keep existing model
            setTranscriptModelConfig((prev: TranscriptModelProps) => ({
                ...prev,
                provider: provider as TranscriptModelProps['provider'],
            }));
            setOpen(false);
            setExpandedProvider(null);
        } else {
            // Expand to show model options
            setExpandedProvider(expandedProvider === provider ? null : provider);
        }
    }

    function handleModelSelect(provider: string, model: string) {
        setTranscriptModelConfig((prev: TranscriptModelProps) => ({
            ...prev,
            provider: provider as TranscriptModelProps['provider'],
            model,
        }));
        setOpen(false);
        setExpandedProvider(null);
    }

    const allProviders = Object.keys(PROVIDER_MODELS);

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger asChild>
                <button
                    className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-muted/60 hover:bg-muted text-muted-foreground hover:text-foreground border border-border/50 transition-colors cursor-pointer select-none"
                    aria-label="Switch transcription model"
                >
                    <Mic className="w-3 h-3 shrink-0" />
                    <span>{displayLabel}</span>
                    {displayModel && (
                        <>
                            <span className="text-muted-foreground/50">·</span>
                            <span className="text-muted-foreground/80">{displayModel}</span>
                        </>
                    )}
                    <ChevronDown className="w-3 h-3 shrink-0 ml-0.5" />
                </button>
            </PopoverTrigger>
            <PopoverContent
                className="w-56 p-1.5"
                align="center"
                side="top"
                sideOffset={6}
            >
                <p className="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground px-2 py-1 mb-0.5">
                    Transcription Provider
                </p>
                <div className="flex flex-col gap-0.5">
                    {allProviders.map((provider) => {
                        const label = PROVIDER_LABELS[provider] ?? provider;
                        const isSelected = provider === currentProvider;
                        const locked = isCloudProviderLocked(provider);
                        const models = PROVIDER_MODELS[provider];
                        const isLocalProv = LOCAL_PROVIDERS.includes(provider as any);
                        const isExpanded = expandedProvider === provider;

                        return (
                            <div key={provider}>
                                <button
                                    className={`w-full flex items-center justify-between px-2 py-1.5 rounded text-sm transition-colors cursor-pointer ${
                                        isSelected
                                            ? 'bg-accent text-accent-foreground font-medium'
                                            : 'hover:bg-muted text-foreground'
                                    }`}
                                    onClick={() => handleProviderClick(provider)}
                                >
                                    <span>{label}</span>
                                    <div className="flex items-center gap-1">
                                        {locked && (
                                            <Lock className="w-3 h-3 text-muted-foreground" />
                                        )}
                                        {!isLocalProv && models.length > 0 && (
                                            <ChevronDown
                                                className={`w-3 h-3 text-muted-foreground transition-transform ${isExpanded ? 'rotate-180' : ''}`}
                                            />
                                        )}
                                    </div>
                                </button>

                                {isExpanded && models.length > 0 && (
                                    <div className="ml-3 mt-0.5 flex flex-col gap-0.5 border-l border-border pl-2">
                                        {models.map((model) => {
                                            const isModelSelected = isSelected && currentModel === model;
                                            return (
                                                <button
                                                    key={model}
                                                    className={`w-full text-left px-2 py-1 rounded text-xs transition-colors cursor-pointer ${
                                                        isModelSelected
                                                            ? 'bg-accent text-accent-foreground font-medium'
                                                            : 'hover:bg-muted text-muted-foreground hover:text-foreground'
                                                    }`}
                                                    onClick={() => handleModelSelect(provider, model)}
                                                >
                                                    {model}
                                                </button>
                                            );
                                        })}
                                    </div>
                                )}
                            </div>
                        );
                    })}
                </div>
            </PopoverContent>
        </Popover>
    );
}
