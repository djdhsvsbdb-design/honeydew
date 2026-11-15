#!/usr/bin/env python3
"""Train/resume Honeydew's autoregressive multi-track event model.

Dataset: JSONL, one song per line. See docs/ai-training.md.
Requires: python -m pip install torch
"""
import argparse, json, math, random
from pathlib import Path
import torch
from torch import nn
from torch.utils.data import Dataset, DataLoader

VOCAB=512
PAD, BOS, EOS = 0, 1, 2
STYLES={"hiphop":3,"trap":4,"drill":5,"phonk":6,"house":7,"other":15}
TRACKS={"drums":8,"bass":9,"chords":10,"lead":11}
BAR_BASE, STEP_BASE, PITCH_BASE, VEL_BASE, DUR_BASE = 16,32,128,256,272

def encode_song(song):
    tokens=[BOS, STYLES.get(str(song.get("style","other")).lower(),15)]
    events=sorted(song["events"],key=lambda e:(int(e.get("bar",0)),int(e.get("step",0)),TRACKS.get(e.get("track"),11),int(e.get("pitch",60))))
    for e in events:
        track=TRACKS.get(str(e.get("track","lead")).lower())
        if track is None: continue
        bar=max(0,min(15,int(e.get("bar",0))))
        step=max(0,min(63,int(e.get("step",0))))
        pitch=max(0,min(127,int(e.get("pitch",60))))
        velocity=max(1,min(127,int(e.get("velocity",100))))
        duration=max(1,min(16,int(e.get("duration",4))))
        tokens.extend([track,BAR_BASE+bar,STEP_BASE+step,PITCH_BASE+pitch,
                       VEL_BASE+min(15,(velocity-1)//8),DUR_BASE+duration-1])
    tokens.append(EOS)
    return tokens

class Songs(Dataset):
    def __init__(self,path,block):
        self.rows=[]
        for line_no,line in enumerate(Path(path).read_text().splitlines(),1):
            if not line.strip(): continue
            try:
                song=json.loads(line)
                if not isinstance(song.get("events"),list) or not song["events"]: continue
                ids=encode_song(song)
                for start in range(0,max(1,len(ids)-1),block):
                    chunk=ids[start:start+block+1]
                    if len(chunk)>=2: self.rows.append(torch.tensor(chunk,dtype=torch.long))
            except Exception as exc:
                raise ValueError(f"{path}:{line_no}: {exc}") from exc
        if not self.rows: raise ValueError(f"No usable songs in {path}")
    def __len__(self): return len(self.rows)
    def __getitem__(self,i): return self.rows[i]

def collate(rows):
    n=max(x.numel() for x in rows)
    x=torch.full((len(rows),n),PAD,dtype=torch.long)
    for i,r in enumerate(rows): x[i,:r.numel()]=r
    return x[:,:-1],x[:,1:]

class GPT(nn.Module):
    def __init__(self,block=512,width=192,layers=6,heads=6,dropout=.1):
        super().__init__()
        self.block=block
        self.tok=nn.Embedding(VOCAB,width,padding_idx=PAD)
        self.pos=nn.Embedding(block,width)
        layer=nn.TransformerEncoderLayer(width,heads,width*4,dropout,batch_first=True,norm_first=True,activation="gelu")
        self.stack=nn.TransformerEncoder(layer,layers,enable_nested_tensor=False)
        self.norm=nn.LayerNorm(width)
        self.head=nn.Linear(width,VOCAB,bias=False)
        self.head.weight=self.tok.weight
    def forward(self,x):
        b,t=x.shape
        if t>self.block: x=x[:,-self.block:]; t=self.block
        mask=torch.triu(torch.ones(t,t,device=x.device,dtype=torch.bool),diagonal=1)
        pad=x.eq(PAD)
        h=self.tok(x)+self.pos(torch.arange(t,device=x.device))[None,:,:]
        h=self.stack(h,mask=mask,src_key_padding_mask=pad)
        return self.head(self.norm(h))

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("--data",required=True); ap.add_argument("--out",default="models/honeydew-music.pt")
    ap.add_argument("--resume",action="store_true"); ap.add_argument("--epochs",type=int,default=30)
    ap.add_argument("--batch-size",type=int,default=16); ap.add_argument("--block",type=int,default=512)
    ap.add_argument("--lr",type=float,default=3e-4); ap.add_argument("--seed",type=int,default=42)
    args=ap.parse_args()
    random.seed(args.seed); torch.manual_seed(args.seed)
    device="cuda" if torch.cuda.is_available() else "cpu"
    ds=Songs(args.data,args.block); loader=DataLoader(ds,batch_size=args.batch_size,shuffle=True,collate_fn=collate)
    model=GPT(args.block).to(device); opt=torch.optim.AdamW(model.parameters(),lr=args.lr,weight_decay=.1)
    start=0
    out=Path(args.out); out.parent.mkdir(parents=True,exist_ok=True)
    if args.resume and out.exists():
        ckpt=torch.load(out,map_location=device,weights_only=False)
        model.load_state_dict(ckpt["model"]); opt.load_state_dict(ckpt["optimizer"]); start=ckpt.get("epoch",0)
    scaler=torch.amp.GradScaler("cuda",enabled=device=="cuda")
    for epoch in range(start,args.epochs):
        model.train(); total=0.0; count=0
        for x,y in loader:
            x,y=x.to(device),y.to(device); opt.zero_grad(set_to_none=True)
            with torch.autocast(device_type="cuda",enabled=device=="cuda"):
                logits=model(x); loss=nn.functional.cross_entropy(logits.reshape(-1,VOCAB),y.reshape(-1),ignore_index=PAD)
            scaler.scale(loss).backward(); scaler.unscale_(opt); nn.utils.clip_grad_norm_(model.parameters(),1.0)
            scaler.step(opt); scaler.update(); total+=loss.item(); count+=1
        avg=total/max(1,count)
        torch.save({"model":model.state_dict(),"optimizer":opt.state_dict(),"epoch":epoch+1,
                    "config":{"vocab":VOCAB,"block":args.block,"width":192,"layers":6,"heads":6},
                    "dataset":str(args.data),"loss":avg},out)
        print(f"epoch {epoch+1}/{args.epochs} loss={avg:.4f} device={device} checkpoint={out}")
if __name__=="__main__": main()
