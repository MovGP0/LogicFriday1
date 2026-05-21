/* 0040e929 FUN_0040e929 */

bool __thiscall FUN_0040e929(void *this,HWND param_1,undefined4 *param_2)

{
  HWND pHVar1;
  bool bVar2;
  
  *(HWND *)this = param_1;
  pHVar1 = CreateWindowExA(0x200,"SysListView32","",0x40000009,0,0,0,0,param_1,(HMENU)0x69,
                           DAT_00452914,(LPVOID)0x0);
  *(HWND *)((int)this + 4) = pHVar1;
  bVar2 = *(int *)((int)this + 4) != 0;
  if (bVar2) {
    SendMessageA(*(HWND *)((int)this + 4),0x1036,0,0x20);
    *param_2 = *(undefined4 *)((int)this + 4);
    _memset((void *)((int)this + 8),0,0x20);
    *(undefined4 *)((int)this + 8) = 7;
    *(undefined4 *)((int)this + 0xc) = 2;
    *(char **)((int)this + 0x14) = "Function";
    *(undefined4 *)((int)this + 0x10) = 0x37;
    SendMessageA(*(HWND *)((int)this + 4),0x101b,0,(int)this + 8);
    *(char **)((int)this + 0x14) = "Inputs";
    *(undefined4 *)((int)this + 0x10) = 0x32;
    SendMessageA(*(HWND *)((int)this + 4),0x101b,1,(int)this + 8);
    *(char **)((int)this + 0x14) = "Outputs";
    *(undefined4 *)((int)this + 0x10) = 0x3c;
    SendMessageA(*(HWND *)((int)this + 4),0x101b,2,(int)this + 8);
    *(undefined **)((int)this + 0x14) = &DAT_0044b90c;
    *(undefined4 *)((int)this + 0x10) = 0x32;
    SendMessageA(*(HWND *)((int)this + 4),0x101b,3,(int)this + 8);
    *(char **)((int)this + 0x14) = "False";
    *(undefined4 *)((int)this + 0x10) = 0x32;
    SendMessageA(*(HWND *)((int)this + 4),0x101b,4,(int)this + 8);
    *(undefined **)((int)this + 0x14) = &DAT_0044b900;
    *(undefined4 *)((int)this + 0x10) = 0x32;
    SendMessageA(*(HWND *)((int)this + 4),0x101b,5,(int)this + 8);
    *(undefined **)((int)this + 0x14) = &DAT_0044b8fc;
    *(undefined4 *)((int)this + 0x10) = 0x46;
    SendMessageA(*(HWND *)((int)this + 4),0x101b,6,(int)this + 8);
    *(char **)((int)this + 0x14) = "Gates";
    *(undefined4 *)((int)this + 0x10) = 0x50;
    SendMessageA(*(HWND *)((int)this + 4),0x101b,7,(int)this + 8);
    *(undefined4 *)((int)this + 0x2c) = 0;
    *(undefined4 *)((int)this + 0x30) = 0;
    *(undefined4 *)((int)this + 0x40) = 9;
    *(undefined4 *)((int)this + 0x28) = 1;
    *(char **)((int)this + 0x3c) = "<none>";
    SendMessageA(*(HWND *)((int)this + 4),0x1007,0,(int)this + 0x28);
  }
  return bVar2;
}
