/* 0041a5b2 FUN_0041a5b2 */

bool __fastcall FUN_0041a5b2(undefined4 *param_1)

{
  BOOL BVar1;
  
  if (param_1[0x30] == 1) {
    param_1[0x23] = param_1[10];
    param_1[0x24] = param_1[0xb];
    param_1[0x25] = param_1[0xc];
    param_1[0x26] = param_1[0xd];
  }
  else {
    param_1[0x23] = param_1[0xe];
    param_1[0x24] = param_1[0xf];
    param_1[0x25] = param_1[0x10];
    param_1[0x26] = param_1[0x11];
  }
  param_1[0x18] = 0x54;
  param_1[0x19] = *param_1;
  param_1[0x1a] = 0;
  param_1[0x1b] = 0;
  param_1[0x1c] = 0x402;
  param_1[0x27] = param_1[2];
  param_1[0x29] = 0;
  param_1[0x2a] = 0;
  param_1[0x2b] = 0;
  param_1[0x2c] = 0;
  BVar1 = PageSetupDlgA((LPPAGESETUPDLGA)(param_1 + 0x18));
  if (BVar1 != 0) {
    param_1[4] = param_1[0x1a];
    param_1[5] = param_1[0x1b];
  }
  return BVar1 != 0;
}
