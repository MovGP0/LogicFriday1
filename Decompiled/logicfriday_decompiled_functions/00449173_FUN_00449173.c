/* 00449173 FUN_00449173 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __cdecl FUN_00449173(int *param_1,int *param_2)

{
  int *piVar1;
  short sVar2;
  int iVar3;
  int *piVar4;
  int *piVar5;
  ushort uVar6;
  uint uVar7;
  int iVar8;
  ushort uVar9;
  uint uVar10;
  ushort uVar11;
  uint unaff_retaddr;
  byte local_2c;
  undefined1 uStack_2b;
  undefined2 uStack_2a;
  short local_28;
  undefined2 uStack_26;
  undefined2 local_24;
  undefined1 uStack_22;
  byte bStack_21;
  int *local_20;
  int local_1c;
  int local_18;
  ushort *local_14;
  ushort *local_10;
  short *local_c;
  uint local_8;
  
  piVar5 = param_2;
  piVar4 = param_1;
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  local_1c = 0;
  local_2c = 0;
  uStack_2b = 0;
  uStack_2a = 0;
  local_28 = 0;
  uStack_26 = 0;
  local_24 = 0;
  uStack_22 = 0;
  bStack_21 = 0;
  uVar10 = *(ushort *)((int)param_2 + 10) & 0x7fff;
  uVar7 = *(ushort *)((int)param_1 + 10) & 0x7fff;
  uVar11 = (*(ushort *)((int)param_2 + 10) ^ *(ushort *)((int)param_1 + 10)) & 0x8000;
  uVar6 = (ushort)uVar7;
  piVar1 = (int *)(uVar10 + uVar7);
  if (((uVar6 < 0x7fff) && (uVar9 = (ushort)uVar10, uVar9 < 0x7fff)) && ((ushort)piVar1 < 0xbffe)) {
    if ((ushort)piVar1 < 0x3fc0) goto LAB_00449223;
    if (uVar6 == 0) {
      piVar1 = (int *)((int)piVar1 + 1);
      uVar6 = 0;
      if ((((param_1[2] & 0x7fffffffU) != 0) || (param_1[1] != 0)) || (*param_1 != 0))
      goto LAB_0044920d;
    }
    else {
LAB_0044920d:
      param_1 = piVar1;
      if (((uVar9 == 0) && (param_1 = (int *)((int)param_1 + 1), (param_2[2] & 0x7fffffffU) == 0))
         && ((param_2[1] == 0 && (*param_2 == 0)))) {
LAB_00449223:
        piVar4[2] = 0;
        piVar4[1] = 0;
        *piVar4 = 0;
        return;
      }
      local_18 = 0;
      local_c = &local_28;
      param_2 = (int *)0x5;
      do {
        if (0 < (int)param_2) {
          local_10 = (ushort *)(local_18 * 2 + (int)piVar4);
          local_14 = (ushort *)(piVar5 + 2);
          local_20 = param_2;
          do {
            iVar8 = ___addl(*(uint *)(local_c + -2),(uint)*local_14 * (uint)*local_10,
                            (uint *)(local_c + -2));
            if (iVar8 != 0) {
              *local_c = *local_c + 1;
            }
            local_10 = local_10 + 1;
            local_14 = local_14 + -1;
            local_20 = (int *)((int)local_20 + -1);
          } while (local_20 != (int *)0x0);
        }
        local_c = local_c + 1;
        local_18 = local_18 + 1;
        param_2 = (int *)((int)param_2 + -1);
      } while (0 < (int)param_2);
      param_1 = (int *)((int)param_1 + 0xc002);
      if ((short)(ushort)param_1 < 1) {
LAB_004492d7:
        iVar8 = (int)param_1 + 0xffff;
        param_1._0_2_ = (ushort)iVar8;
        if ((short)(ushort)param_1 < 0) {
          uVar7 = -iVar8;
          uVar10 = uVar7 & 0xffff;
          param_1._0_2_ = (ushort)param_1 + (short)uVar7;
          do {
            if ((local_2c & 1) != 0) {
              local_1c = local_1c + 1;
            }
            ___shr_12((uint *)&local_2c);
            uVar10 = uVar10 - 1;
          } while (uVar10 != 0);
          if (local_1c != 0) {
            local_2c = local_2c | 1;
          }
        }
      }
      else {
        do {
          if ((bStack_21 & 0x80) != 0) break;
          ___shl_12((uint *)&local_2c);
          param_1 = (int *)((int)param_1 + 0xffff);
        } while (0 < (short)(ushort)param_1);
        if ((short)(ushort)param_1 < 1) goto LAB_004492d7;
      }
      if ((0x8000 < CONCAT11(uStack_2b,local_2c)) ||
         (sVar2 = CONCAT11(bStack_21,uStack_22), iVar3 = CONCAT22(local_24,uStack_26),
         iVar8 = CONCAT22(local_28,uStack_2a),
         (CONCAT22(uStack_2a,CONCAT11(uStack_2b,local_2c)) & 0x1ffff) == 0x18000)) {
        if (CONCAT22(local_28,uStack_2a) == -1) {
          iVar8 = 0;
          if (CONCAT22(local_24,uStack_26) == -1) {
            if (CONCAT11(bStack_21,uStack_22) == -1) {
              param_1._0_2_ = (ushort)param_1 + 1;
              sVar2 = -0x8000;
              iVar3 = 0;
              iVar8 = 0;
            }
            else {
              sVar2 = CONCAT11(bStack_21,uStack_22) + 1;
              iVar3 = 0;
              iVar8 = 0;
            }
          }
          else {
            sVar2 = CONCAT11(bStack_21,uStack_22);
            iVar3 = CONCAT22(local_24,uStack_26) + 1;
          }
        }
        else {
          iVar8 = CONCAT22(local_28,uStack_2a) + 1;
          sVar2 = CONCAT11(bStack_21,uStack_22);
          iVar3 = CONCAT22(local_24,uStack_26);
        }
      }
      local_28 = (short)((uint)iVar8 >> 0x10);
      uStack_2a = (undefined2)iVar8;
      local_24 = (undefined2)((uint)iVar3 >> 0x10);
      uStack_26 = (undefined2)iVar3;
      bStack_21 = (byte)((ushort)sVar2 >> 8);
      uStack_22 = (undefined1)sVar2;
      if (0x7ffe < (ushort)param_1) goto LAB_00449380;
      *(undefined2 *)piVar4 = uStack_2a;
      *(uint *)((int)piVar4 + 2) = CONCAT22(uStack_26,local_28);
      *(uint *)((int)piVar4 + 6) = CONCAT13(bStack_21,CONCAT12(uStack_22,local_24));
      uVar6 = (ushort)param_1 | uVar11;
    }
    *(ushort *)((int)piVar4 + 10) = uVar6;
  }
  else {
LAB_00449380:
    piVar4[1] = 0;
    *piVar4 = 0;
    piVar4[2] = (-(uint)(uVar11 != 0) & 0x80000000) + 0x7fff8000;
  }
  return;
}
