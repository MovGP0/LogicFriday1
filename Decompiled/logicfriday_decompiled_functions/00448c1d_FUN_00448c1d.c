/* 00448c1d FUN_00448c1d */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __cdecl
FUN_00448c1d(int param_1,uint param_2,short *param_3,int param_4,byte param_5,short *param_6)

{
  short *psVar1;
  char cVar2;
  uint uVar3;
  short *psVar4;
  short *psVar5;
  short sVar6;
  int iVar7;
  uint unaff_retaddr;
  char *pcVar8;
  uint local_34;
  undefined4 uStack_30;
  undefined4 uStack_2c;
  undefined1 local_28;
  undefined1 local_27;
  undefined1 local_26;
  undefined1 local_25;
  undefined1 local_24;
  undefined1 local_23;
  undefined1 local_22;
  undefined1 local_21;
  undefined1 local_20;
  undefined1 local_1f;
  undefined1 local_1e;
  undefined1 local_1d;
  undefined2 local_1c;
  undefined4 local_1a;
  undefined4 local_16;
  undefined1 local_12;
  char cStack_11;
  int local_10;
  undefined4 local_c;
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  uVar3 = (uint)param_3 & 0x7fff;
  local_28 = 0xcc;
  local_27 = 0xcc;
  local_26 = 0xcc;
  local_25 = 0xcc;
  local_24 = 0xcc;
  local_23 = 0xcc;
  local_22 = 0xcc;
  local_21 = 0xcc;
  local_20 = 0xcc;
  local_1f = 0xcc;
  local_1e = 0xfb;
  local_1d = 0x3f;
  local_c = 1;
  if (((uint)param_3 & 0x8000) == 0) {
    *(undefined1 *)(param_6 + 1) = 0x20;
  }
  else {
    *(undefined1 *)(param_6 + 1) = 0x2d;
  }
  if ((((short)uVar3 == 0) && (param_2 == 0)) && (param_1 == 0)) {
LAB_00448d96:
    *(undefined1 *)(param_6 + 2) = 0x30;
LAB_00448e9c:
    *param_6 = 0;
    *(undefined1 *)((int)param_6 + 5) = 0;
    *(undefined1 *)(param_6 + 1) = 0x20;
    *(undefined1 *)((int)param_6 + 3) = 1;
    local_c = 1;
  }
  else {
    if ((short)uVar3 == 0x7fff) {
      *param_6 = 1;
      if (((param_2 == 0x80000000) && (param_1 == 0)) || ((param_2 & 0x40000000) != 0)) {
        if ((((uint)param_3 & 0x8000) == 0) || (param_2 != 0xc0000000)) {
          if ((param_2 != 0x80000000) || (param_1 != 0)) goto LAB_00448d0c;
          pcVar8 = "1#INF";
        }
        else {
          if (param_1 != 0) {
LAB_00448d0c:
            pcVar8 = "1#QNAN";
            goto LAB_00448d11;
          }
          pcVar8 = "1#IND";
        }
        FUN_0043ebd0((uint *)(param_6 + 2),(uint *)pcVar8);
        *(undefined1 *)((int)param_6 + 3) = 5;
      }
      else {
        pcVar8 = "1#SNAN";
LAB_00448d11:
        FUN_0043ebd0((uint *)(param_6 + 2),(uint *)pcVar8);
        *(undefined1 *)((int)param_6 + 3) = 6;
      }
      return 0;
    }
    local_1c = 0;
    sVar6 = (short)(((uVar3 >> 8) + (param_2 >> 0x18) * 2) * 0x4d + -0x134312f4 + uVar3 * 0x4d10 >>
                   0x10);
    local_12 = (undefined1)uVar3;
    cStack_11 = (char)(uVar3 >> 8);
    local_1a = param_1;
    local_16 = param_2;
    FUN_004493ab((int *)&local_1c,-(int)sVar6,1);
    if (0x3ffe < CONCAT11(cStack_11,local_12)) {
      sVar6 = sVar6 + 1;
      FUN_00449173((int *)&local_1c,(int *)&local_28);
    }
    *param_6 = sVar6;
    if (((param_5 & 1) != 0) && (param_4 = param_4 + sVar6, param_4 < 1)) goto LAB_00448d96;
    if (0x15 < param_4) {
      param_4 = 0x15;
    }
    iVar7 = CONCAT11(cStack_11,local_12) - 0x3ffe;
    local_12 = 0;
    cStack_11 = '\0';
    param_3 = (short *)0x8;
    do {
      ___shl_12((uint *)&local_1c);
      param_3 = (short *)((int)param_3 + -1);
    } while (param_3 != (short *)0x0);
    if (iVar7 < 0) {
      for (uVar3 = -iVar7 & 0xff; uVar3 != 0; uVar3 = uVar3 - 1) {
        ___shr_12((uint *)&local_1c);
      }
    }
    local_10 = param_4 + 1;
    psVar4 = param_6 + 2;
    param_3 = psVar4;
    iVar7 = local_1a;
    uVar3 = local_16;
    if (0 < local_10) {
      do {
        local_16._2_2_ = (undefined2)(uVar3 >> 0x10);
        local_16._0_2_ = (undefined2)uVar3;
        local_1a._2_2_ = (undefined2)((uint)iVar7 >> 0x10);
        local_1a._0_2_ = (undefined2)iVar7;
        local_34 = CONCAT22((undefined2)local_1a,local_1c);
        uStack_30 = CONCAT22((undefined2)local_16,local_1a._2_2_);
        uStack_2c = CONCAT13(cStack_11,CONCAT12(local_12,local_16._2_2_));
        local_1a = iVar7;
        local_16 = uVar3;
        ___shl_12((uint *)&local_1c);
        ___shl_12((uint *)&local_1c);
        ___add_12((uint *)&local_1c,&local_34);
        ___shl_12((uint *)&local_1c);
        cVar2 = cStack_11;
        cStack_11 = '\0';
        psVar4 = (short *)((int)param_3 + 1);
        local_10 = local_10 + -1;
        *(char *)param_3 = cVar2 + '0';
        param_3 = psVar4;
        iVar7 = local_1a;
        uVar3 = local_16;
      } while (local_10 != 0);
    }
    psVar5 = psVar4 + -1;
    psVar1 = param_6 + 2;
    if (*(char *)((int)psVar4 + -1) < '5') {
      for (; (psVar1 <= psVar5 && ((char)*psVar5 == '0')); psVar5 = (short *)((int)psVar5 + -1)) {
      }
      if (psVar5 < psVar1) {
        *(char *)psVar1 = '0';
        goto LAB_00448e9c;
      }
    }
    else {
      for (; (psVar1 <= psVar5 && ((char)*psVar5 == '9')); psVar5 = (short *)((int)psVar5 + -1)) {
        *(char *)psVar5 = '0';
      }
      if (psVar5 < psVar1) {
        psVar5 = (short *)((int)psVar5 + 1);
        *param_6 = *param_6 + 1;
      }
      *(char *)psVar5 = (char)*psVar5 + '\x01';
    }
    cVar2 = ((char)psVar5 - (char)param_6) + -3;
    *(char *)((int)param_6 + 3) = cVar2;
    *(undefined1 *)(cVar2 + 4 + (int)param_6) = 0;
  }
  return local_c;
}
