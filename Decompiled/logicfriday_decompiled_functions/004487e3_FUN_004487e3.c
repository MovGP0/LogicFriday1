/* 004487e3 FUN_004487e3 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __cdecl
FUN_004487e3(undefined2 *param_1,int *param_2,byte *param_3,int param_4,int param_5,int param_6,
            int param_7)

{
  int iVar1;
  int iVar2;
  uint uVar3;
  byte bVar4;
  undefined4 uVar5;
  char *pcVar6;
  byte *pbVar7;
  byte *pbVar8;
  uint unaff_retaddr;
  char local_5c [23];
  char local_45;
  undefined2 local_40;
  undefined2 uStack_3e;
  undefined2 uStack_3c;
  undefined4 local_3a;
  ushort local_36;
  undefined4 local_30;
  int local_2c;
  int local_28;
  int local_24;
  int local_20;
  undefined4 local_1c;
  int local_18;
  int local_14;
  uint local_10;
  char *local_c;
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  iVar1 = 0;
  pcVar6 = local_5c;
  local_30 = 0;
  local_20 = 1;
  local_10 = 0;
  local_18 = 0;
  local_2c = 0;
  local_28 = 0;
  local_24 = 0;
  local_14 = 0;
  local_1c = 0;
  for (pbVar7 = param_3;
      (((bVar4 = *pbVar7, bVar4 == 0x20 || (bVar4 == 9)) || (bVar4 == 10)) ||
      (local_c = pcVar6, bVar4 == 0xd)); pbVar7 = pbVar7 + 1) {
  }
LAB_0044883d:
  bVar4 = *pbVar7;
  pbVar8 = pbVar7 + 1;
  switch(iVar1) {
  case 0:
    if (('0' < (char)bVar4) && ((char)bVar4 < ':')) {
LAB_0044885a:
      iVar1 = 3;
      goto LAB_00448a7b;
    }
    if (bVar4 == DAT_00452434) {
LAB_00448869:
      iVar1 = 5;
      pbVar7 = pbVar8;
      goto LAB_0044883d;
    }
    if (bVar4 == 0x2b) {
      local_30 = 0;
      iVar1 = 2;
      pbVar7 = pbVar8;
      goto LAB_0044883d;
    }
    if (bVar4 == 0x2d) {
      iVar1 = 2;
      local_30 = 0x8000;
      pbVar7 = pbVar8;
      goto LAB_0044883d;
    }
    if (bVar4 != 0x30) {
      local_c = pcVar6;
      goto LAB_00448a39;
    }
    break;
  case 1:
    iVar1 = 1;
    local_18 = 1;
    if (('0' < (char)bVar4) && ((char)bVar4 < ':')) goto LAB_0044885a;
    if (bVar4 == DAT_00452434) goto LAB_004488bb;
    if ((bVar4 == 0x2b) || (bVar4 == 0x2d)) goto LAB_004488fc;
    pbVar7 = pbVar8;
    if (bVar4 != 0x30) goto LAB_004488d5;
    goto LAB_0044883d;
  case 2:
    if (('0' < (char)bVar4) && ((char)bVar4 < ':')) goto LAB_0044885a;
    if (bVar4 == DAT_00452434) goto LAB_00448869;
    pbVar7 = param_3;
    if (bVar4 != 0x30) goto LAB_00448a36;
    break;
  case 3:
    local_18 = 1;
    while (iVar1 = _isdigit((uint)bVar4), iVar1 != 0) {
      if (local_10 < 0x19) {
        local_10 = local_10 + 1;
        *pcVar6 = bVar4 - 0x30;
        pcVar6 = pcVar6 + 1;
      }
      else {
        local_14 = local_14 + 1;
      }
      bVar4 = *pbVar8;
      pbVar8 = pbVar8 + 1;
    }
    if (bVar4 != DAT_00452434) goto LAB_004489ab;
LAB_004488bb:
    iVar1 = 4;
    pbVar7 = pbVar8;
    goto LAB_0044883d;
  case 4:
    local_18 = 1;
    local_2c = 1;
    if (local_10 == 0) {
      while (bVar4 == 0x30) {
        local_14 = local_14 + -1;
        bVar4 = *pbVar8;
        pbVar8 = pbVar8 + 1;
      }
    }
    while (iVar1 = _isdigit((uint)bVar4), iVar1 != 0) {
      if (local_10 < 0x19) {
        local_10 = local_10 + 1;
        *pcVar6 = bVar4 - 0x30;
        pcVar6 = pcVar6 + 1;
        local_14 = local_14 + -1;
      }
      bVar4 = *pbVar8;
      pbVar8 = pbVar8 + 1;
    }
LAB_004489ab:
    if ((bVar4 == 0x2b) || (bVar4 == 0x2d)) {
LAB_004488fc:
      iVar1 = 0xb;
      pbVar7 = pbVar8 + -1;
    }
    else {
LAB_004488d5:
      if (((char)bVar4 < 'D') ||
         (('E' < (char)bVar4 && (((char)bVar4 < 'd' || ('e' < (char)bVar4)))))) {
LAB_00448a35:
        pbVar7 = pbVar8 + -1;
        goto LAB_00448a36;
      }
      iVar1 = 6;
      pbVar7 = pbVar8;
    }
    goto LAB_0044883d;
  case 5:
    local_2c = 1;
    iVar1 = _isdigit((uint)bVar4);
    pbVar7 = param_3;
    if (iVar1 != 0) {
      iVar1 = 4;
      goto LAB_00448a7b;
    }
    goto LAB_00448a36;
  case 6:
    pbVar7 = pbVar7 + -1;
    param_3 = pbVar7;
    if (((char)bVar4 < '1') || ('9' < (char)bVar4)) {
      if (bVar4 == 0x2b) goto LAB_00448ab0;
      if (bVar4 == 0x2d) goto LAB_00448aa4;
      if (bVar4 != 0x30) goto LAB_00448aca;
      goto LAB_00448a13;
    }
LAB_00448a79:
    iVar1 = 9;
LAB_00448a7b:
    pbVar7 = pbVar8 + -1;
    goto LAB_0044883d;
  case 7:
    if (('0' < (char)bVar4) && ((char)bVar4 < ':')) goto LAB_00448a79;
    pbVar7 = param_3;
    if (bVar4 != 0x30) goto LAB_00448a36;
LAB_00448a13:
    iVar1 = 8;
    pbVar7 = pbVar8;
    goto LAB_0044883d;
  case 8:
    local_28 = 1;
    while (bVar4 == 0x30) {
      bVar4 = *pbVar8;
      pbVar8 = pbVar8 + 1;
    }
    if (('0' < (char)bVar4) && ((char)bVar4 < ':')) goto LAB_00448a79;
    goto LAB_00448a35;
  case 9:
    local_28 = 1;
    iVar1 = 0;
    local_c = pcVar6;
    goto LAB_00448af7;
  default:
    goto switchD_00448849_caseD_a;
  case 0xb:
    if (param_7 != 0) {
      param_3 = pbVar7;
      if (bVar4 == 0x2b) {
LAB_00448ab0:
        iVar1 = 7;
        pbVar7 = pbVar8;
      }
      else {
        if (bVar4 != 0x2d) {
LAB_00448aca:
          local_c = pcVar6;
          goto LAB_00448a39;
        }
LAB_00448aa4:
        local_20 = -1;
        iVar1 = 7;
        pbVar7 = pbVar8;
      }
      goto LAB_0044883d;
    }
    iVar1 = 10;
    pbVar8 = pbVar7;
switchD_00448849_caseD_a:
    pbVar7 = pbVar8;
    if (iVar1 != 10) goto LAB_0044883d;
LAB_00448a36:
    local_c = pcVar6;
    goto LAB_00448a39;
  }
  iVar1 = 1;
  pbVar7 = pbVar8;
  goto LAB_0044883d;
LAB_00448af7:
  iVar2 = _isdigit((uint)bVar4);
  if (iVar2 == 0) goto LAB_00448b1a;
  iVar1 = (char)bVar4 + -0x30 + iVar1 * 10;
  if (0x1450 < iVar1) {
    iVar1 = 0x1451;
    goto LAB_00448b1a;
  }
  bVar4 = *pbVar8;
  pbVar8 = pbVar8 + 1;
  goto LAB_00448af7;
LAB_00448b1a:
  while( true ) {
    local_24 = iVar1;
    iVar1 = _isdigit((uint)bVar4);
    if (iVar1 == 0) break;
    bVar4 = *pbVar8;
    pbVar8 = pbVar8 + 1;
    iVar1 = local_24;
  }
  pbVar7 = pbVar8 + -1;
LAB_00448a39:
  *param_2 = (int)pbVar7;
  if (local_18 == 0) {
    local_1c = 4;
  }
  else {
    pcVar6 = local_c;
    if (0x18 < local_10) {
      if ('\x04' < local_45) {
        local_45 = local_45 + '\x01';
      }
      pcVar6 = local_c + -1;
      local_14 = local_14 + 1;
      local_10 = 0x18;
    }
    if (local_10 != 0) {
      while (pcVar6 = pcVar6 + -1, *pcVar6 == '\0') {
        local_10 = local_10 - 1;
        local_14 = local_14 + 1;
      }
      FUN_004486ff(local_5c,local_10,(uint *)&local_40);
      iVar1 = local_24;
      if (local_20 < 0) {
        iVar1 = -local_24;
      }
      uVar3 = iVar1 + local_14;
      if (local_28 == 0) {
        uVar3 = uVar3 + param_5;
      }
      if (local_2c == 0) {
        uVar3 = uVar3 - param_6;
      }
      if (0x1450 < (int)uVar3) {
        uVar5 = 0;
        local_36 = 0x7fff;
        local_3a = 0x80000000;
        local_40 = 0;
        local_1c = 2;
        goto LAB_00448bc7;
      }
      if (-0x1451 < (int)uVar3) {
        FUN_004493ab((int *)&local_40,uVar3,param_4);
        uVar5 = CONCAT22(uStack_3c,uStack_3e);
        goto LAB_00448bc7;
      }
      local_1c = 1;
    }
  }
  local_40 = 0;
  local_36 = 0;
  local_3a = 0;
  uVar5 = 0;
LAB_00448bc7:
  *(undefined4 *)(param_1 + 1) = uVar5;
  *(undefined4 *)(param_1 + 3) = local_3a;
  param_1[5] = local_36 | (ushort)local_30;
  *param_1 = local_40;
  return local_1c;
}
