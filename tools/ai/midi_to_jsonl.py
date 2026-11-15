#!/usr/bin/env python3
"""Convert a folder of MIDI files into Honeydew's JSONL event dataset."""
import argparse, json, re
from pathlib import Path
import mido

def classify(name, channel):
    s=name.lower()
    if channel == 9 or re.search(r"drum|kick|snare|perc|808",s): return "drums"
    if re.search(r"bass|sub",s): return "bass"
    if re.search(r"chord|pad|piano|keys|rhodes",s): return "chords"
    return "lead"

def convert(path, style):
    mid=mido.MidiFile(path)
    tpq=mid.ticks_per_beat
    tempo_events=[]
    events=[]
    for ti,track in enumerate(mid.tracks):
        name=next((m.name for m in track if m.type=="track_name"),f"track_{ti}")
        abs_tick=0; active={}
        for msg in track:
            abs_tick += msg.time
            if msg.type=="set_tempo": tempo_events.append((abs_tick,msg.tempo))
            if msg.type not in ("note_on","note_off"): continue
            key=(msg.channel,msg.note)
            if msg.type=="note_on" and msg.velocity>0:
                active.setdefault(key,[]).append((abs_tick,msg.velocity))
            else:
                starts=active.get(key,[])
                if not starts: continue
                start,vel=starts.pop(0)
                # Convert MIDI ticks to sixteenth-note grid, assuming standard 4/4.
                step_abs=max(0,round(start/(tpq/4)))
                end_abs=max(step_abs+1,round(abs_tick/(tpq/4)))
                events.append({"track":classify(name,msg.channel),"bar":step_abs//16,
                               "step":step_abs%16,"pitch":int(msg.note),"velocity":int(vel),
                               "duration":max(1,min(16,end_abs-step_abs))})
    if not events: return None
    # The file may contain tempo changes; JSONL v1 stores one representative BPM.
    # Use the tempo active at the beginning, not whichever track happened to be read last.
    tempo=min(tempo_events,key=lambda item:item[0])[1] if tempo_events else 500000
    bpm=round(60_000_000/tempo)
    return {"style":style,"bpm":bpm,"source":path.name,"events":sorted(events,key=lambda e:(e["bar"],e["step"],e["track"],e["pitch"]))}

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("input",help="MIDI file or folder")
    ap.add_argument("--out",required=True)
    ap.add_argument("--style",default="other",choices=["hiphop","trap","drill","phonk","house","other"])
    args=ap.parse_args()
    src=Path(args.input)
    if not src.exists(): raise FileNotFoundError(f"Input path does not exist: {src}")
    files=[src] if src.is_file() else sorted(src.rglob("*.mid"))+sorted(src.rglob("*.midi"))
    count=0
    with Path(args.out).open("w",encoding="utf-8") as f:
        for p in files:
            try:
                song=convert(p,args.style)
                if song:
                    f.write(json.dumps(song,ensure_ascii=False)+"\n"); count+=1
            except Exception as exc: print(f"skip {p}: {exc}")
    print(f"wrote {count} songs to {args.out}")
if __name__=="__main__": main()
