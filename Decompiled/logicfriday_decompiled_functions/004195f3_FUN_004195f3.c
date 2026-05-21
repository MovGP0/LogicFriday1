/* 004195f3 FUN_004195f3 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_004195f3(void *this,WPARAM *param_1)

{
  size_t sVar1;
  uint unaff_retaddr;
  uint local_1c;
  char local_18 [12];
  uint local_c;
  uint local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  FUN_00417cde((int)this);
  _memset((void *)((int)this + 0x20),0,0x20);
  *(undefined4 *)((int)this + 0x20) = 7;
  *(undefined4 *)((int)this + 0x30) = 0x1f;
  *(undefined **)((int)this + 0x2c) = &DAT_0044c990;
  *(undefined4 *)((int)this + 0x28) = 0x28;
  *(undefined4 *)((int)this + 0x24) = 2;
  SendMessageA(*(HWND *)((int)this + 0x14),0x101a,0,(int)this + 0x20);
  *(undefined4 *)((int)this + 0x28) = 0x19;
  for (local_8 = 1; local_8 <= param_1[0x31]; local_8 = local_8 + 1) {
    sVar1 = _strlen((char *)((int)param_1 + (local_8 - 1) * 9 + 0x160));
    if (sVar1 == 1) {
      FUN_0043ed39(local_18,&DAT_0044c978);
      *(char **)((int)this + 0x2c) = local_18;
    }
    else {
      *(uint *)((int)this + 0x2c) = (int)param_1 + (local_8 - 1) * 9 + 0x160;
    }
    *(undefined4 *)((int)this + 0x24) = 2;
    SendMessageA(*(HWND *)((int)this + 0x14),0x101b,local_8,(int)this + 0x20);
  }
  local_8 = local_8 + 1;
  *(undefined4 *)((int)this + 0x28) = 0x32;
  *(undefined **)((int)this + 0x2c) = &DAT_0044c970;
  SendMessageA(*(HWND *)((int)this + 0x14),0x101b,local_8,(int)this + 0x20);
  local_8 = local_8 + 1;
  *(undefined4 *)((int)this + 0x28) = 0x32;
  for (local_1c = 0; local_1c < param_1[0x32]; local_1c = local_1c + 1) {
    sVar1 = _strlen((char *)((int)param_1 + local_1c * 9 + 0xd0));
    if (sVar1 == 1) {
      FUN_0043ed39(local_18,&DAT_0044c978);
      *(char **)((int)this + 0x2c) = local_18;
    }
    else {
      *(uint *)((int)this + 0x2c) = (int)param_1 + local_1c * 9 + 0xd0;
    }
    SendMessageA(*(HWND *)((int)this + 0x14),0x101b,local_8 + local_1c,(int)this + 0x20);
  }
  *(WPARAM *)((int)this + 0x68) = param_1[0x31] + 2 + param_1[0x32];
  *(undefined4 *)((int)this + 0x28) = 0;
  SendMessageA(*(HWND *)((int)this + 0x14),0x101b,*(int *)((int)this + 0x68) + 1,(int)this + 0x20);
  for (local_8 = 0; local_8 < *(uint *)((int)this + 0x68); local_8 = local_8 + 1) {
    SendMessageA(*(HWND *)((int)this + 0x14),0x101e,local_8,0xfffe);
  }
  SendMessageA(*(HWND *)((int)this + 0x14),0x102f,*param_1,0);
  SendMessageA(*(HWND *)((int)this + 0x14),0x1013,0,0);
  return 0;
}
