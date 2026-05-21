/* 004461e9 FUN_004461e9 */

uint FUN_004461e9(void)

{
  undefined4 *puVar1;
  int iVar2;
  undefined4 *puVar3;
  uint uVar4;
  int *piVar5;
  int local_8;
  int local_4;
  
  uVar4 = 0xffffffff;
  iVar2 = FUN_00441ceb(0xb);
  if (iVar2 != 0) {
    __lock(0xb);
    local_8 = 0;
    local_4 = 0;
    piVar5 = &DAT_0046cc40;
    while (puVar3 = (undefined4 *)*piVar5, puVar1 = puVar3, puVar3 != (undefined4 *)0x0) {
      for (; puVar3 < puVar1 + 0x120; puVar3 = puVar3 + 9) {
        if ((*(byte *)(puVar3 + 1) & 1) == 0) {
          if (puVar3[2] == 0) {
            __lock(10);
            if (puVar3[2] == 0) {
              iVar2 = ___crtInitCritSecAndSpinCount(puVar3 + 3,4000);
              if (iVar2 == 0) {
                FUN_00441cd6(10);
                goto LAB_00446322;
              }
              puVar3[2] = puVar3[2] + 1;
            }
            FUN_00441cd6(10);
          }
          EnterCriticalSection((LPCRITICAL_SECTION)(puVar3 + 3));
          if ((*(byte *)(puVar3 + 1) & 1) == 0) {
            *puVar3 = 0xffffffff;
            uVar4 = ((int)puVar3 - *piVar5) / 0x24 + local_4;
            if (uVar4 != 0xffffffff) goto LAB_00446325;
            break;
          }
          LeaveCriticalSection((LPCRITICAL_SECTION)(puVar3 + 3));
        }
        puVar1 = (undefined4 *)*piVar5;
      }
      local_4 = local_4 + 0x20;
      local_8 = local_8 + 1;
      piVar5 = piVar5 + 1;
      if (0x46cd3f < (int)piVar5) goto LAB_00446325;
    }
    puVar3 = _malloc(0x480);
    if (puVar3 != (undefined4 *)0x0) {
      DAT_0046cc2c = DAT_0046cc2c + 0x20;
      (&DAT_0046cc40)[local_8] = puVar3;
      puVar1 = puVar3;
      for (; puVar3 < puVar1 + 0x120; puVar3 = puVar3 + 9) {
        *(undefined1 *)(puVar3 + 1) = 0;
        *puVar3 = 0xffffffff;
        puVar3[2] = 0;
        *(undefined1 *)((int)puVar3 + 5) = 10;
        puVar1 = (undefined4 *)(&DAT_0046cc40)[local_8];
      }
      uVar4 = local_8 << 5;
      iVar2 = FUN_00446154(uVar4);
      if (iVar2 == 0) {
LAB_00446322:
        uVar4 = 0xffffffff;
      }
    }
LAB_00446325:
    FUN_00441cd6(0xb);
  }
  return uVar4;
}
