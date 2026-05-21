/* 00417d61 FUN_00417d61 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_00417d61(void *this,uint *param_1)

{
  int iVar1;
  void *pvVar2;
  size_t sVar3;
  uint unaff_retaddr;
  uint local_28;
  uint local_24;
  char local_1c [12];
  uint local_10;
  uint local_c;
  int local_8;
  
  local_10 = DAT_00451a00 ^ unaff_retaddr;
  local_8 = 0;
  FUN_00417c56((int)this);
  *(undefined4 *)((int)this + 0x90) = 0;
  *(undefined4 *)((int)this + 0x8c) = 0;
  if (param_1[0x90] == 0) {
    pvVar2 = _realloc(*(void **)((int)this + 0x7c),*param_1 * 0x48);
    *(void **)((int)this + 0x7c) = pvVar2;
    for (local_c = 0; local_c < *param_1; local_c = local_c + 1) {
      local_28 = 0;
      while( true ) {
        iVar1 = local_8;
        if (param_1[0x32] <= local_28) goto LAB_00417dcc;
        if ((param_1[0x92] != 0) || (*(int *)(param_1[local_28 + 0x21] + local_c * 4) != 0)) break;
        local_28 = local_28 + 1;
      }
      for (local_24 = 0; iVar1 = local_8 + 1, local_24 < param_1[0x32]; local_24 = local_24 + 1) {
        *(uint *)(local_8 * 0x48 + *(int *)((int)this + 0x7c)) = local_c;
        *(undefined4 *)(*(int *)((int)this + 0x7c) + 4 + local_8 * 0x48) = 0;
        *(undefined4 *)(local_8 * 0x48 + *(int *)((int)this + 0x7c) + 8 + local_24 * 4) =
             *(undefined4 *)(param_1[local_24 + 0x21] + local_c * 4);
      }
LAB_00417dcc:
      local_8 = iVar1;
    }
    *(int *)((int)this + 0x74) = local_8;
    *(undefined4 *)((int)this + 0x8c) = 1;
    *(undefined4 *)((int)this + 0x90) = 1;
    for (local_c = 0; local_c < param_1[0x32]; local_c = local_c + 1) {
      if (param_1[local_c + 1] != *param_1) {
        *(undefined4 *)((int)this + 0x90) = 0;
      }
      if (param_1[local_c + 1] != 0) {
        *(undefined4 *)((int)this + 0x8c) = 0;
      }
    }
    if (((1000 < *param_1) && (*(int *)((int)this + 0x8c) == 0)) &&
       (*(int *)((int)this + 0x90) == 0)) {
      pvVar2 = _realloc(*(void **)((int)this + 0x7c),local_8 * 0x48);
      *(void **)((int)this + 0x7c) = pvVar2;
    }
    *(uint *)((int)this + 0x80) = param_1[0x95];
  }
  else if (param_1[0x7d] == 0) {
    *(undefined4 *)((int)this + 0x8c) = 1;
  }
  else {
    pvVar2 = _realloc(*(void **)((int)this + 0x7c),param_1[0x7d] * 0x48);
    *(void **)((int)this + 0x7c) = pvVar2;
    _memset(*(void **)((int)this + 0x7c),0,param_1[0x7d] * 0x48);
    for (local_c = 0; local_c < param_1[0x7d]; local_c = local_c + 1) {
      *(undefined4 *)(local_c * 0x48 + *(int *)((int)this + 0x7c)) =
           *(undefined4 *)(param_1[0x7e] + 4 + local_c * 0xc);
      *(undefined4 *)(*(int *)((int)this + 0x7c) + 4 + local_c * 0x48) =
           *(undefined4 *)(param_1[0x7e] + 8 + local_c * 0xc);
      for (local_28 = 0; local_28 < param_1[0x32]; local_28 = local_28 + 1) {
        *(undefined4 *)(local_c * 0x48 + *(int *)((int)this + 0x7c) + 8 + local_28 * 4) =
             *(undefined4 *)(param_1[local_28 + 0x7f] + local_c * 4);
      }
    }
    *(uint *)((int)this + 0x74) = param_1[0x7d];
  }
  _memset((void *)((int)this + 0x20),0,0x20);
  *(undefined4 *)((int)this + 0x20) = 7;
  *(undefined4 *)((int)this + 0x30) = 0x1f;
  *(undefined4 *)((int)this + 0x28) = 0x19;
  *(undefined4 *)((int)this + 0x24) = 2;
  for (local_c = 0; local_c < param_1[0x31]; local_c = local_c + 1) {
    sVar3 = _strlen((char *)((int)param_1 + local_c * 9 + 0x160));
    if (sVar3 == 1) {
      FUN_0043ed39(local_1c,&DAT_0044c978);
      *(char **)((int)this + 0x2c) = local_1c;
    }
    else {
      *(uint *)((int)this + 0x2c) = (int)param_1 + local_c * 9 + 0x160;
    }
    if (local_c == 0) {
      SendMessageA(*(HWND *)((int)this + 0x10),0x101a,0,(int)this + 0x20);
    }
    else {
      SendMessageA(*(HWND *)((int)this + 0x10),0x101b,local_c,(int)this + 0x20);
    }
  }
  local_c = local_c + 1;
  *(undefined4 *)((int)this + 0x28) = 0x32;
  *(undefined **)((int)this + 0x2c) = &DAT_0044c970;
  SendMessageA(*(HWND *)((int)this + 0x10),0x101b,local_c,(int)this + 0x20);
  local_c = local_c + 1;
  *(undefined4 *)((int)this + 0x28) = 0x32;
  for (local_28 = 0; local_28 < param_1[0x32]; local_28 = local_28 + 1) {
    sVar3 = _strlen((char *)((int)param_1 + local_28 * 9 + 0xd0));
    if (sVar3 == 1) {
      FUN_0043ed39(local_1c,&DAT_0044c978);
      *(char **)((int)this + 0x2c) = local_1c;
    }
    else {
      *(uint *)((int)this + 0x2c) = (int)param_1 + local_28 * 9 + 0xd0;
    }
    SendMessageA(*(HWND *)((int)this + 0x10),0x101b,local_c + local_28,(int)this + 0x20);
  }
  *(uint *)((int)this + 0x6c) = param_1[0x31] + 1 + param_1[0x32];
  *(undefined4 *)((int)this + 0x28) = 0;
  SendMessageA(*(HWND *)((int)this + 0x10),0x101b,*(int *)((int)this + 0x6c) + 1,(int)this + 0x20);
  for (local_c = 0; local_c < *(uint *)((int)this + 0x6c); local_c = local_c + 1) {
    SendMessageA(*(HWND *)((int)this + 0x10),0x101e,local_c,0xfffe);
  }
  if ((*(int *)((int)this + 0x8c) == 0) && (*(int *)((int)this + 0x90) == 0)) {
    SendMessageA(*(HWND *)((int)this + 0x10),0x102f,*(WPARAM *)((int)this + 0x74),0);
  }
  else {
    SendMessageA(*(HWND *)((int)this + 0x10),0x102f,1,0);
  }
  *(uint *)((int)this + 0x70) = param_1[0x31];
  *(uint *)((int)this + 0x84) = param_1[0x90];
  *(uint *)((int)this + 0x88) = param_1[0x92];
  return 0;
}
