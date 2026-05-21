/* 0040eb81 FUN_0040eb81 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_0040eb81(void *this,uint *param_1)

{
  uint unaff_retaddr;
  undefined1 local_5a0 [12];
  undefined4 local_594;
  undefined4 local_590;
  undefined1 local_578 [12];
  undefined4 local_56c;
  undefined4 local_568;
  undefined1 local_550 [8];
  undefined4 local_548;
  char *local_53c;
  undefined1 local_528 [8];
  undefined4 local_520;
  char *local_514;
  undefined1 local_500 [8];
  undefined4 local_4f8;
  uint *local_4ec;
  undefined1 local_4d8 [8];
  undefined4 local_4d0;
  uint *local_4c4;
  undefined1 local_4b0 [8];
  undefined4 local_4a8;
  uint *local_49c;
  undefined1 local_488 [8];
  undefined4 local_480;
  uint *local_474;
  undefined1 local_460 [8];
  undefined4 local_458;
  uint *local_44c;
  WPARAM local_438;
  uint local_434;
  WPARAM local_430;
  uint local_42c [8];
  uint local_40c [257];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  local_434 = param_1[0x44];
  *(undefined4 *)((int)this + 0x2c) = *(undefined4 *)((int)this + 0x50);
  *(int *)((int)this + 0x50) = *(int *)((int)this + 0x50) + 1;
  *(undefined4 *)((int)this + 0x30) = 0;
  *(undefined4 *)((int)this + 0x40) = 9;
  *(undefined4 *)((int)this + 0x28) = 5;
  local_430 = *(int *)(local_434 + 200);
  if (local_430 == 1) {
    FUN_0043ed39((char *)local_40c,&DAT_0044b974);
  }
  else {
    FUN_0043ed39((char *)local_40c,(byte *)"%s-%s");
  }
  if (DAT_00452eb8 == 0) {
    local_430 = FUN_0040be9e((char *)local_40c);
    if (local_430 != 0) {
      FUN_0043ed39((char *)local_42c,&DAT_0044b964);
      FUN_0043ebe0(local_40c,local_42c);
    }
  }
  FUN_0043ebd0(param_1,local_40c);
  *(uint **)((int)this + 0x3c) = local_40c;
  *(uint *)((int)this + 0x48) = param_1[0x44];
  if (*(int *)((int)this + 0x58) != 0) {
    SendMessageA(*(HWND *)((int)this + 4),0x1008,0,0);
    *(undefined4 *)((int)this + 0x58) = 0;
  }
  local_438 = SendMessageA(*(HWND *)((int)this + 4),0x1007,0,(int)this + 0x28);
  FUN_0043ed39((char *)local_40c,&DAT_0044b960);
  local_458 = 1;
  local_44c = local_40c;
  SendMessageA(*(HWND *)((int)this + 4),0x102e,local_438,(LPARAM)local_460);
  FUN_0043ed39((char *)local_40c,&DAT_0044b960);
  local_480 = 2;
  local_474 = local_40c;
  SendMessageA(*(HWND *)((int)this + 4),0x102e,local_438,(LPARAM)local_488);
  FUN_0043ed39((char *)local_40c,&DAT_0044b960);
  for (local_430 = 1; (int)local_430 < *(int *)(local_434 + 200); local_430 = local_430 + 1) {
    FUN_0043ed39((char *)local_42c,&DAT_0044b958);
    FUN_0043ebe0(local_40c,local_42c);
  }
  local_4a8 = 3;
  local_49c = local_40c;
  SendMessageA(*(HWND *)((int)this + 4),0x102e,local_438,(LPARAM)local_4b0);
  FUN_0043ed39((char *)local_40c,&DAT_0044b960);
  for (local_430 = 1; (int)local_430 < *(int *)(local_434 + 200); local_430 = local_430 + 1) {
    FUN_0043ed39((char *)local_42c,&DAT_0044b958);
    FUN_0043ebe0(local_40c,local_42c);
  }
  local_4d0 = 4;
  local_4c4 = local_40c;
  SendMessageA(*(HWND *)((int)this + 4),0x102e,local_438,(LPARAM)local_4d8);
  FUN_0043ed39((char *)local_40c,&DAT_0044b960);
  for (local_430 = 1; (int)local_430 < *(int *)(local_434 + 200); local_430 = local_430 + 1) {
    FUN_0043ed39((char *)local_42c,&DAT_0044b958);
    FUN_0043ebe0(local_40c,local_42c);
  }
  local_4f8 = 5;
  local_4ec = local_40c;
  SendMessageA(*(HWND *)((int)this + 4),0x102e,local_438,(LPARAM)local_500);
  local_520 = 6;
  local_514 = "Unminimized";
  SendMessageA(*(HWND *)((int)this + 4),0x102e,local_438,(LPARAM)local_528);
  local_548 = 7;
  local_53c = "Not mapped";
  SendMessageA(*(HWND *)((int)this + 4),0x102e,local_438,(LPARAM)local_550);
  for (local_430 = 0; (int)local_430 < *(int *)((int)this + 0x50); local_430 = local_430 + 1) {
    local_568 = 2;
    local_56c = 0;
    SendMessageA(*(HWND *)((int)this + 4),0x102b,local_430,(LPARAM)local_578);
  }
  local_590 = 2;
  local_594 = 2;
  SendMessageA(*(HWND *)((int)this + 4),0x102b,local_438,(LPARAM)local_5a0);
  *(WPARAM *)((int)this + 0x54) = local_430;
  SendMessageA(*(HWND *)((int)this + 4),0x1013,local_438,0);
  return 1;
}
