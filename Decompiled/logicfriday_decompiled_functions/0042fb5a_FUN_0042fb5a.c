/* 0042fb5a FUN_0042fb5a */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_0042fb5a(void *this,int param_1)

{
  size_t sVar1;
  int iVar2;
  char *pcVar3;
  undefined4 *puVar4;
  char *pcVar5;
  undefined4 *puVar6;
  uint unaff_retaddr;
  int local_2c;
  char local_28 [28];
  uint local_c;
  int local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  pcVar3 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  pcVar5 = local_28;
  for (iVar2 = 6; iVar2 != 0; iVar2 = iVar2 + -1) {
    *(undefined4 *)pcVar5 = *(undefined4 *)pcVar3;
    pcVar3 = pcVar3 + 4;
    pcVar5 = pcVar5 + 4;
  }
  *(undefined2 *)pcVar5 = *(undefined2 *)pcVar3;
  pcVar5[2] = pcVar3[2];
  FUN_0042fcbc();
  if (*(uint *)((int)this + 0x2350) < *(uint *)((int)this + 0x1650)) {
    *(int *)((int)this + 0x2350) = *(int *)((int)this + 0x16c4) + 1000;
  }
  if (*(uint *)((int)this + 0x16c0) < *(uint *)((int)this + 0x16c8)) {
    *(int *)((int)this + 0x16c0) = *(int *)((int)this + 0x16c8) + 1000;
  }
  *(undefined4 *)((int)this + 0x234c) = *(undefined4 *)(param_1 + 0x234c);
  puVar4 = (undefined4 *)(param_1 + 0xc4);
  puVar6 = (undefined4 *)((int)this + 0xc4);
  for (iVar2 = 0x4b; iVar2 != 0; iVar2 = iVar2 + -1) {
    *puVar6 = *puVar4;
    puVar4 = puVar4 + 1;
    puVar6 = puVar6 + 1;
  }
  local_8 = 0;
  do {
    if (*(int *)((int)this + 0xc4) <= local_8) {
      return 0;
    }
    for (local_2c = 0; local_2c < 0x1a; local_2c = local_2c + 1) {
      sVar1 = _strlen((char *)((int)this + local_8 * 9 + 0x160));
      if ((sVar1 == 1) && (*(char *)((int)this + local_8 * 9 + 0x160) == local_28[local_2c])) {
        *(undefined4 *)((int)this + local_2c * 4 + 0x25ec) = 1;
        break;
      }
    }
    local_8 = local_8 + 1;
  } while( true );
}
