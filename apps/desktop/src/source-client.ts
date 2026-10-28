import {invoke} from '@tauri-apps/api/core';
import type {Message,Session} from './data';
export type Scan={sessions:Session[];total:number;excluded:number;observedAt:number;sourcePath:string;warnings:string[]};
export type SourceDetail={messages:Message[];observedAt:number;warnings:string[];truncated:boolean};
async function call<T>(command:string,endpoint:string,args:Record<string,string>={}):Promise<T>{
  if('__TAURI_INTERNALS__' in window)return invoke<T>(command,args);
  const response=await fetch(`/api/local-source/${endpoint}`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(args)});
  if(!response.ok){let reason='本机 Rust 数据源服务未启动，请启动来源服务或使用桌面版';try{const value=await response.json();if(typeof value.error==='string')reason=value.error;}catch{}throw Error(reason);}
  return response.json();
}
export const scanDefaultSource=()=>call<Scan>('scan_default_source','scan');
export const readSourceSession=(id:string)=>call<SourceDetail>('read_source_session','session',{id});
export const openSourceSession=(id:string)=>call('open_source_session','open',{id});
export const openSourceLink=(url:string)=>call('open_chatgpt_link','open-link',{url});
export function mergeImported(previous:Session[],incoming:Session[]):Session[]{
  const map=new Map(previous.map(s=>[s.id,s]));
  for(const item of incoming){const old=map.get(item.id);map.set(item.id,old?{...item,purpose:old.purpose,project:old.project,projectName:old.projectName,status:old.status,archived:old.archived,tasks:old.tasks,source:{...item.source!,loaded:false}}:item);}
  return [...map.values()];
}
export function safeChatLink(value:string):string|undefined{
  if(!value.trim())return undefined;
  const url=new URL(value);
  if(url.protocol!=='https:'||url.hostname!=='chatgpt.com'||url.username||url.password||url.port||!/^\/(?:c\/[\w-]+|g\/[\w-]+\/c\/[\w-]+)\/?$/.test(url.pathname))throw Error('请填写 https://chatgpt.com/c/… 形式的原会话链接，不接受脚本、分享链接或其他网站');
  return value.trim();
}
