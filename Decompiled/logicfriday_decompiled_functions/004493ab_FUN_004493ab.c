/* 004493ab FUN_004493ab */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __cdecl FUN_004493ab(int *param_1,uint param_2,int param_3)

{
  uint uVar1;
  uint uVar2;
  undefined4 *puVar3;
  ushort *puVar4;
  uint unaff_retaddr;
  ushort local_14;
  undefined4 local_12;
  undefined2 uStack_e;
  undefined4 uStack_c;
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  puVar3 = &DAT_00452570;
  if (param_2 != 0) {
    if ((int)param_2 < 0) {
      param_2 = -param_2;
      puVar3 = (undefined4 *)0x4526d0;
    }
    if (param_3 == 0) {
      *(undefined2 *)param_1 = 0;
    }
    while (param_2 != 0) {
      uVar1 = (int)param_2 >> 3;
      uVar2 = param_2 & 7;
      puVar3 = puVar3 + 0x15;
      param_2 = uVar1;
      if (uVar2 != 0) {
        puVar4 = (ushort *)(puVar3 + uVar2 * 3);
        if (0x7fff < *puVar4) {
          local_14 = (ushort)*(undefined4 *)puVar4;
          local_12._0_2_ = (undefined2)((uint)*(undefined4 *)puVar4 >> 0x10);
          local_12._2_2_ = (undefined2)*(undefined4 *)(puVar4 + 2);
          uStack_e = (undefined2)((uint)*(undefined4 *)(puVar4 + 2) >> 0x10);
          uStack_c = *(undefined4 *)(puVar4 + 4);
          local_12 = CONCAT22(local_12._2_2_,(undefined2)local_12) + -1;
          puVar4 = &local_14;
        }
        FUN_00449173(param_1,(int *)puVar4);
      }
    }
  }
  return;
}
