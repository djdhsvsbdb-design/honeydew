"""Local arrangement composer for Honeydew.

Uses a configured OpenAI-compatible chat endpoint only to interpret the brief.
The note generator is deterministic/procedural so the feature works offline too;
it is not a trained MIDI-GPT model.
"""
import json
import os
import random
import urllib.request


def _classify(prompt, tempo_override=None):
    text = prompt.lower()
    brief = {
        "genre": "house",
        "mood": "uplifting",
        "mode": "minor",
        "tempo": 124,
        "density": "medium",
    }
    if any(word in text for word in ("trance", "uplifting", "anthem")):
        brief.update(genre="trance", tempo=138, mood="uplifting")
    elif any(word in text for word in ("techno", "industrial")):
        brief.update(genre="techno", tempo=130, mood="driving")
    elif any(word in text for word in ("lofi", "lo-fi", "chill", "jazzy")):
        brief.update(genre="lofi", tempo=84, mood="relaxed", mode="major")
    elif any(word in text for word in ("ambient", "cinematic", "soundtrack")):
        brief.update(genre="ambient", tempo=92, mood="atmospheric")
    elif any(word in text for word in ("dnb", "drum and bass", "jungle")):
        brief.update(genre="dnb", tempo=174, mood="energetic")
    if any(word in text for word in ("happy", "bright", "major", "cheerful")):
        brief["mode"] = "major"
    if any(word in text for word in ("dark", "sad", "minor", "melancholic")):
        brief["mode"] = "minor"
    if any(word in text for word in ("sparse", "minimal", "simple")):
        brief["density"] = "low"
    if any(word in text for word in ("busy", "complex", "energetic", "dense")):
        brief["density"] = "high"
    api_key = os.environ.get("HONEYDEW_AI_API_KEY")
    if api_key:
        endpoint = os.environ.get(
            "HONEYDEW_AI_BASE_URL", "https://api.openai.com/v1/chat/completions"
        )
        model = os.environ.get("HONEYDEW_AI_MODEL", "gpt-4o-mini")
        payload = {
            "model": model,
            "temperature": 0.1,
            "response_format": {"type": "json_object"},
            "messages": [
                {"role": "system", "content":
                 "Classify the user's music brief. Return JSON only with genre "
                 "(house, trance, techno, lofi, ambient, dnb), mood (uplifting, "
                 "driving, relaxed, atmospheric, energetic), mode (major or minor), "
                 "tempo (60-190 integer), density (low, medium, high). Do not add notes."},
                {"role": "user", "content": prompt[:2000]},
            ],
        }
        req = urllib.request.Request(
            endpoint,
            data=json.dumps(payload).encode(),
            headers={"Authorization": "Bearer " + api_key,
                     "Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=20) as response:
            result = json.loads(response.read().decode())
        parsed = json.loads(result["choices"][0]["message"]["content"])
        allowed = {
            "genre": ("house", "trance", "techno", "lofi", "ambient", "dnb"),
            "mood": ("uplifting", "driving", "relaxed", "atmospheric", "energetic"),
            "mode": ("major", "minor"),
            "density": ("low", "medium", "high"),
        }
        for key, choices in allowed.items():
            if parsed.get(key) in choices:
                brief[key] = parsed[key]
        if isinstance(parsed.get("tempo"), (int, float)):
            brief["tempo"] = max(60, min(190, int(parsed["tempo"])))
    if tempo_override is not None:
        brief["tempo"] = max(60, min(190, int(tempo_override)))
    return brief


def _pick(available, preferences, fallback):
    names = [str(x) for x in available]
    for preference in preferences:
        for name in names:
            if preference.lower() in name.lower():
                return name
    return fallback if fallback in names else (names[0] if names else fallback)


def _effect_name(catalogue, *words):
    for effect in catalogue:
        name = effect.get("name", "")
        if any(word in name.lower() for word in words):
            return name
    return None


def _effect(name, mix):
    return {"name": name, "changes": [{"index": -2, "value": mix}]}


def build_plan(prompt, bars, seed, instruments, effect_catalogue, tempo_override=None,
               beats_per_bar=4.0):
    bars = max(1, min(16, int(bars)))
    bpb = float(beats_per_bar)
    if not 1.0 <= bpb <= 12.0:
        bpb = 4.0
    rng = random.Random(int(seed))
    brief = _classify(prompt, tempo_override)
    available = instruments.get("built_in", [])
    drums = "Drums" if "Drums" in available else _pick(available, ["drum"], "Drums")
    bass = _pick(available, ["Transistor Bass", "MiniSynth", "Bass"], "MiniSynth")
    chords = _pick(available, ["SuperSaw", "MiniSynth", "3x Osc"], "MiniSynth")
    lead = _pick(available, ["SuperSaw", "MiniSynth", "Transistor Bass"], "MiniSynth")
    minor = brief["mode"] == "minor"
    scale = [0, 2, 3, 5, 7, 8, 10] if minor else [0, 2, 4, 5, 7, 9, 11]
    progression = [0, 5, 3, 6] if minor else [0, 4, 5, 3]
    if brief["genre"] == "ambient":
        density = 2
    elif brief["density"] == "low":
        density = 4
    elif brief["density"] == "high":
        density = 8
    else:
        density = 6

    drum_notes = []
    if brief["genre"] != "ambient":
        for bar in range(bars):
            base = bar * bpb
            beats = max(1, int(bpb))
            for beat in range(beats):
                drum_notes.append({"beat": base + beat * bpb / beats, "length_beats": 0.18,
                                   "key": 0, "velocity": 190 if beat % 2 == 0 else 145})
            for beat in (1, 3):
                if beat < bpb:
                    drum_notes.append({"beat": base + beat * bpb / 4.0, "length_beats": 0.18,
                                       "key": 1, "velocity": 170})
            hat_step = 0.5 if brief["genre"] != "lofi" else 1.0
            step = 0.0
            while step < bpb:
                drum_notes.append({"beat": base + step, "length_beats": 0.12,
                                   "key": 3, "velocity": rng.randint(65, 105)})
                step += hat_step
            if brief["genre"] in ("house", "trance", "dnb") and bpb >= 4:
                drum_notes.append({"beat": base + bpb - 0.5, "length_beats": 0.12,
                                   "key": 4, "velocity": 85})

    bass_notes, chord_notes, lead_notes = [], [], []
    for bar in range(bars):
        chord_degree = progression[bar % len(progression)]
        root = 48 + scale[chord_degree]
        chord_intervals = [0, 3, 7] if minor else [0, 4, 7]
        if brief["genre"] in ("trance", "ambient"):
            chord_intervals.append(12)
        for interval in chord_intervals:
            chord_notes.append({"beat": bar * bpb, "length_beats": bpb * 0.95,
                                "key": min(96, root + interval),
                                "velocity": 105 if interval else 125})
        if brief["genre"] == "ambient":
            bass_notes.append({"beat": bar * bpb, "length_beats": bpb * 0.92,
                               "key": root - 12, "velocity": 105})
        else:
            for step in range(density):
                beat = step * (bpb / density)
                key = root - 12 if step % 4 else root - 12 + 7
                bass_notes.append({"beat": bar * bpb + beat,
                                   "length_beats": (bpb / density) * 0.9,
                                   "key": key, "velocity": rng.randint(105, 155)})
        if brief["genre"] != "ambient":
            for step in range(4 if brief["density"] == "low" else 8):
                degree = rng.choice([1, 2, 4, 5, 6])
                octave = 12 if step % 3 == 0 else 0
                beat = step * (bpb / (4 if brief["density"] == "low" else 8))
                key = 60 + scale[degree] + octave
                lead_notes.append({"beat": bar * bpb + beat,
                                   "length_beats": (bpb / (4 if brief["density"] == "low" else 8)) * 0.7,
                                   "key": key, "velocity": rng.randint(80, 125)})

    tracks = [
        {"target": {"kind": "new", "instrument": drums, "name": "AI Drums"},
         "start_bar": 0, "bars": bars, "notes": drum_notes},
        {"target": {"kind": "new", "instrument": bass, "name": "AI Bass"},
         "start_bar": 0, "bars": bars, "notes": bass_notes},
        {"target": {"kind": "new", "instrument": chords, "name": "AI Chords"},
         "start_bar": 0, "bars": bars, "notes": chord_notes},
        {"target": {"kind": "new", "instrument": lead, "name": "AI Lead"},
         "start_bar": 0, "bars": bars, "notes": lead_notes},
    ]
    reverb = _effect_name(effect_catalogue, "reverb")
    delay = _effect_name(effect_catalogue, "delay", "echo")
    compressor = _effect_name(effect_catalogue, "compressor", "limiter")
    chorus = _effect_name(effect_catalogue, "chorus")
    designs = [
        [_effect(compressor, 0.65)] if compressor and brief["genre"] != "ambient" else [],
        [_effect(compressor, 0.45)] if compressor else [],
        ([_effect(reverb, 0.3)] if reverb else []) +
        ([_effect(chorus, 0.25)] if chorus else []),
        ([_effect(reverb, 0.28)] if reverb else []) +
        ([_effect(delay, 0.2)] if delay else []),
    ]
    return {
        "brief": brief,
        "arrangement": {"tempo_bpm": brief["tempo"], "tracks": tracks},
        "sound_design": designs,
    }


def _instrument_changes(parameters, role):
    targets = {
        "bass": {"cutoff": 0.46, "drive": 0.34, "attack": 0.08, "release": 0.24},
        "chords": {"attack": 0.28, "release": 0.62, "cutoff": 0.72},
        "lead": {"attack": 0.12, "release": 0.48, "cutoff": 0.68},
    }.get(role, {})
    changes = []
    used = set()
    for param in parameters:
        name = str(param.get("name", "")).lower()
        if param.get("kind") != "continuous" or param.get("index", 0) == 0:
            continue
        for token, value in targets.items():
            if token in name and token not in used:
                changes.append({"index": param["index"], "value": value})
                used.add(token)
                break
    return changes


def compose(prompt, bars, seed, tempo_override, honeydew_call):
    song = honeydew_call("song")
    instruments = honeydew_call("instruments")
    effects = honeydew_call("effect_catalog")
    song = song.get("result", song)
    instruments = instruments.get("result", instruments)
    effects = effects.get("result", effects)
    plan = build_plan(
        prompt, bars, seed, instruments, effects.get("effects", []), tempo_override,
        song.get("beats_per_bar", 4.0),
    )
    applied = honeydew_call("arrange", plan["arrangement"])
    if applied.get("error"):
        raise RuntimeError(applied["error"])
    created = applied.get("result", {}).get("created", [])
    warnings = []
    roles = ("drums", "bass", "chords", "lead")
    for index, channel in enumerate(created):
        role = roles[index] if index < len(roles) else "lead"
        if role != "drums":
            params_result = honeydew_call(
                "instrument_params", None, channel["channel_id"]
            )
            params_result = params_result.get("result", params_result)
            modules = params_result.get("modules", [])
            if modules:
                changes = _instrument_changes(modules[0].get("parameters", []), role)
                if changes:
                    tuned = honeydew_call("set_instrument_params", {
                        "channel_id": channel["channel_id"],
                        "module_id": modules[0]["module_id"],
                        "changes": changes,
                    })
                    if tuned.get("error"):
                        warnings.append(channel["channel_name"] + ": " + tuned["error"])
        design = plan["sound_design"][index]
        if design:
            result = honeydew_call("apply_sound_design", {
                "channel_id": channel["channel_id"], "effects": design
            })
            if result.get("error"):
                warnings.append(channel["channel_name"] + ": " + result["error"])
    return {
        "brief": plan["brief"],
        "created_tracks": created,
        "effects_added": sum(len(x) for x in plan["sound_design"]),
        "warnings": warnings,
        "note": "Procedural composition generated from the brief. Configure HONEYDEW_AI_API_KEY to enable AI brief interpretation; this is not a trained MIDI model.",
    }
