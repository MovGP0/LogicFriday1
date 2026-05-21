/* 00430039 FUN_00430039 */

void __fastcall FUN_00430039(int param_1)

{
  undefined4 uVar1;
  void *pvVar2;
  size_t sVar3;
  int iVar4;
  undefined4 *puVar5;
  undefined4 *puVar6;
  int local_c;
  int local_8;
  
  *(undefined4 *)(param_1 + 0x165c) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x165c);
  *(undefined4 *)(param_1 + 0x1660) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x1660);
  if (1 < *(uint *)(param_1 + 0x165c)) {
    pvVar2 = _realloc(*(void **)(param_1 + 0x268),*(int *)(param_1 + 0x165c) * 0x7fff);
    *(void **)(param_1 + 0x268) = pvVar2;
  }
  if (1 < *(uint *)(param_1 + 0x1660)) {
    pvVar2 = _realloc(*(void **)(param_1 + 0x26c),*(int *)(param_1 + 0x1660) * 0x7fff);
    *(void **)(param_1 + 0x26c) = pvVar2;
  }
  FUN_0043ebd0(*(uint **)(param_1 + 0x268),*(uint **)(*(int *)(param_1 + 0x23cc) + 0x268));
  FUN_0043ebd0(*(uint **)(param_1 + 0x26c),*(uint **)(*(int *)(param_1 + 0x23cc) + 0x26c));
  if (*(int *)(*(int *)(param_1 + 0x23cc) + 0x270) != 0) {
    sVar3 = _strlen(*(char **)(*(int *)(param_1 + 0x23cc) + 0x270));
    pvVar2 = _malloc(sVar3 + 1);
    *(void **)(param_1 + 0x270) = pvVar2;
    FUN_0043ebd0(*(uint **)(param_1 + 0x270),*(uint **)(*(int *)(param_1 + 0x23cc) + 0x270));
  }
  puVar5 = (undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x1f0);
  puVar6 = (undefined4 *)(param_1 + 0x1f0);
  for (iVar4 = 0x13; iVar4 != 0; iVar4 = iVar4 + -1) {
    *puVar6 = *puVar5;
    puVar5 = puVar5 + 1;
    puVar6 = puVar6 + 1;
  }
  pvVar2 = _malloc(*(int *)(param_1 + 500) * 0xc);
  *(void **)(param_1 + 0x1f8) = pvVar2;
  for (local_8 = 0; local_8 < *(int *)(param_1 + 500); local_8 = local_8 + 1) {
    puVar5 = (undefined4 *)(*(int *)(*(int *)(param_1 + 0x23cc) + 0x1f8) + local_8 * 0xc);
    puVar6 = (undefined4 *)(*(int *)(param_1 + 0x1f8) + local_8 * 0xc);
    *puVar6 = *puVar5;
    puVar6[1] = puVar5[1];
    puVar6[2] = puVar5[2];
  }
  for (local_8 = 0; local_8 < *(int *)(param_1 + 200); local_8 = local_8 + 1) {
    pvVar2 = _malloc(*(int *)(param_1 + 500) << 2);
    *(void **)(param_1 + 0x1fc + local_8 * 4) = pvVar2;
    for (local_c = 0; local_c < *(int *)(param_1 + 500); local_c = local_c + 1) {
      *(undefined4 *)(*(int *)(param_1 + 0x1fc + local_8 * 4) + local_c * 4) =
           *(undefined4 *)(*(int *)(*(int *)(param_1 + 0x23cc) + 0x1fc + local_8 * 4) + local_c * 4)
      ;
    }
  }
  FUN_0043ebd0(*(uint **)(param_1 + 0x274),*(uint **)(*(int *)(param_1 + 0x23cc) + 0x274));
  puVar5 = (undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x3a8);
  puVar6 = (undefined4 *)(param_1 + 0x3a8);
  for (iVar4 = 0x4aa; iVar4 != 0; iVar4 = iVar4 + -1) {
    *puVar6 = *puVar5;
    puVar5 = puVar5 + 1;
    puVar6 = puVar6 + 1;
  }
  *(undefined4 *)(param_1 + 0x23c) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x23c);
  *(undefined4 *)(param_1 + 0x240) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x240);
  *(undefined4 *)(param_1 + 0x244) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x244);
  *(undefined4 *)(param_1 + 0x260) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x260);
  *(undefined4 *)(param_1 + 0x248) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x248);
  *(undefined4 *)(param_1 + 0x250) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x250);
  *(undefined4 *)(param_1 + 0x254) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x254);
  *(undefined4 *)(param_1 + 0x16b4) = 1;
  *(undefined4 *)(param_1 + 0x16b8) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x16b8);
  *(undefined4 *)(param_1 + 0x16bc) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x16bc);
  *(undefined4 *)(param_1 + 0x24c) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x24c);
  *(undefined4 *)(param_1 + 0x264) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x264);
  *(undefined4 *)(param_1 + 0x2308) = 0;
  iVar4 = *(int *)(param_1 + 0x23cc);
  *(undefined4 *)(param_1 + 0x26f0) = *(undefined4 *)(iVar4 + 0x26f0);
  *(undefined4 *)(param_1 + 0x26f4) = *(undefined4 *)(iVar4 + 0x26f4);
  *(undefined4 *)(param_1 + 0x26f8) = *(undefined4 *)(iVar4 + 0x26f8);
  *(undefined4 *)(param_1 + 0x26fc) = *(undefined4 *)(iVar4 + 0x26fc);
  for (local_8 = 0; local_8 < 0x1a; local_8 = local_8 + 1) {
    *(undefined4 *)(param_1 + 0x25ec + local_8 * 4) =
         *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x25ec + local_8 * 4);
  }
  uVar1 = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x1674);
  *(undefined4 *)(param_1 + 0x1670) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x1670);
  *(undefined4 *)(param_1 + 0x1674) = uVar1;
  *(undefined4 *)(param_1 + 0x1678) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 0x1678);
  *(undefined4 *)(param_1 + 600) = *(undefined4 *)(*(int *)(param_1 + 0x23cc) + 600);
  *(undefined4 *)(param_1 + 0x25c) = 1;
  *(undefined4 *)(param_1 + 0x2668) = 0;
  return;
}
